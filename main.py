import asyncio
import os
import socket

import decky_plugin  # type: ignore
from settings import SettingsManager  # type: ignore

settings = SettingsManager(
    name="deck-screenshot-explorer-settings",
    settings_directory=os.environ["DECKY_PLUGIN_SETTINGS_DIR"]
)
settings.read()


def is_port_in_use(port: int | str) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        return s.connect_ex(("localhost", int(port))) == 0


class Plugin:
    backend: asyncio.subprocess.Process | None = None
    server_running = False
    _watchdog_task = None
    error: str | None = None

    async def watchdog(self):
        while True:
            await asyncio.sleep(1)
            try:
                if not self.backend:
                    continue
                if self.backend.returncode is None:
                    continue
                await Plugin.start_server(self, False)
            except Exception as e:
                decky_plugin.logger.error(f"Watchdog error: {e}", exc_info=e)
                raise e

    async def kill_server(self) -> None:
        """Kill the Rust backend server

        Okay, okay, okay, let's kill backend, let's kill this backend, let's beat him to death with a SIGKILL.
        """

        if self.backend:
            decky_plugin.logger.info("kill_server: Stopping Rust backend...")
            self.backend.terminate()
            kill_this_man = False
            try:
                asyncio.wait_for(self.backend.wait(), timeout=10.0)
            except asyncio.TimeoutError:
                decky_plugin.logger.error("kill_server: Rust backend did not stop in time, killing it")
                kill_this_man = True
            if kill_this_man and self.backend.returncode is None:
                self.backend.kill()
                try:
                    asyncio.wait_for(self.backend.wait(), timeout=10.0)
                except asyncio.TimeoutError:
                    decky_plugin.logger.error("kill_server: Rust backend is not responding, giving up")
            self.backend = None
            self.server_running = False
            decky_plugin.logger.info("kill_server: Rust backend stopped")

    async def start_server(self, enable: bool = True) -> bool:
        """Start or stop the Rust backend server

        Parameters:
            enable (bool): Start or stop the server

        Returns:
            bool: True if the server is running, False otherwise
        """
        try:
            Plugin.set_error(self, None)
            if enable == self.server_running:
                decky_plugin.logger.info("start_server: Server already running")
                return True
            if enable:
                use_port = await Plugin.get_port(self)
                if is_port_in_use(use_port):
                    Plugin.set_error(self, "Port is already in use")
                    return False
                decky_plugin.logger.info("start_server: Starting Rust backend...")
                self.backend = await asyncio.create_subprocess_shell(
                    f"{decky_plugin.DECKY_PLUGIN_DIR}/bin/backend",
                    env={
                        "HOST": "0.0.0.0",
                        "PORT": str(use_port),
                        "DECKY_PLUGIN_DIR": decky_plugin.DECKY_PLUGIN_DIR,
                        "DECKY_LOG_INTO": decky_plugin.DECKY_PLUGIN_LOG_DIR,
                        "HOME": decky_plugin.HOME,
                    },
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.PIPE,
                )
                self.server_running = True
                decky_plugin.logger.info("start_server: Rust backend started")
                return True
            else:
                await Plugin.kill_server(self)
                return False
        except Exception as e:
            decky_plugin.logger.error(f"Error starting/stopping server: {e}", exc_info=e)
            raise e

    async def get_port(self) -> int:
        return settings.getSetting("PORT", 5158)

    async def set_port(self, port: int) -> int:
        settings.setSetting("PORT", int(port))
        settings.commit()
        return port

    async def get_error(self) -> str | None:
        return self.error

    def set_error(self, error: str | None) -> None:
        self.error = error

    async def get_accepted_warning(self) -> bool:
        return settings.getSetting("ACCEPTED_WARNING", False)

    async def set_accepted_warning(self) -> None:
        decky_plugin.logger.info("Accepted warning")
        settings.setSetting("ACCEPTED_WARNING", True)
        settings.commit()

    async def get_ip_address(self):
        return socket.gethostbyname(socket.gethostname())

    async def get_server_running(self):
        return self.server_running

    async def get_status(self):
        return {
            "server_running": await Plugin.get_server_running(self),
            "ip_address": await Plugin.get_ip_address(self),
            "port": await Plugin.get_port(self),
            "accepted_warning": await Plugin.get_accepted_warning(self),
            "error": await Plugin.get_error(self),
        }

    # Asyncio-compatible long-running code, executed in a task when the plugin is loaded
    async def _main(self):
        try:
            if settings.getSetting("PORT") is None:
                await Plugin.set_port(self, 5158)

            decky_plugin.logger.info("deck-screenshot-explorer: loading plugin...")
            loop = asyncio.get_event_loop()
            self._watchdog_task = loop.create_task(Plugin.watchdog(self))
            decky_plugin.logger.info("deck-screenshot-explorer: plugin loaded")
        except Exception as e:
            decky_plugin.logger.error(f"Error loading plugin: {e}", exc_info=e)
            raise e

    # Function called first during the unload process, utilize this to handle your plugin being removed
    async def _unload(self):
        decky_plugin.logger.info("deck-screenshot-explorer: unloading plugin...")
        await Plugin.start_server(self, False)
        if self._watchdog_task:
            self._watchdog_task.cancel()
        decky_plugin.logger.info("deck-screenshot-explorer: plugin unloaded")
