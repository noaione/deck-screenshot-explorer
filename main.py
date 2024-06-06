import asyncio
import os
import socket
import subprocess

import decky_plugin  # type: ignore
from settings import SettingsManager  # type: ignore

settings = SettingsManager(
    name="deck-screenshot-explorer-settings",
    settings_directory=os.environ["DECKY_PLUGIN_SETTINGS_DIR"]
)
settings.read()


def is_port_in_use(port: int | str) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        return s.connect_ex(("localhost", str(port))) == 0


class Plugin:
    backend: subprocess.Popen[bytes] | None = None
    server_running = False
    _watchdog_task = None
    error: str | None = None

    async def _watchdog(self):
        while True:
            try:
                if not self.backend:
                    await asyncio.sleep(1)
                    continue
                if self.backend.poll() is None:
                    await asyncio.sleep(1)
                    continue
                await self.start_server(False)
                await asyncio.sleep(1)
            except Exception as e:
                decky_plugin.logger.error(f"Watchdog error: {e}")
                raise e

    async def start_server(self, enable: bool = True) -> bool:
        """Start or stop the Rust backend server

        Parameters:
            enable (bool): Start or stop the server

        Returns:
            bool: True if the server is running, False otherwise
        """
        try:
            self.error = None
            if enable == self.server_running:
                decky_plugin.logger.info("start_server: Server already running")
                return True
            if enable:
                use_port = await self.get_port()
                if is_port_in_use(use_port):
                    self.set_error("Port is already in use")
                    return False
                decky_plugin.logger.info("start_server: Starting Rust backend...")
                self.backend = subprocess.Popen(  # noqa: ASYNC101
                    [
                        "HOST=0.0.0.0",
                        f"PORT={use_port}",
                        f"{decky_plugin.DECKY_PLUGIN_DIR}/bin/backend",
                    ],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                )
                self.server_running = True
                decky_plugin.logger.info("start_server: Rust backend started")
                return True
            else:
                if self.backend:
                    decky_plugin.logger.info("start_server: Stopping Rust backend...")
                    self.backend.terminate()
                    self.backend = None
                    self.server_running = False
                    decky_plugin.logger.info("start_server: Rust backend stopped")
                return False
        except Exception as e:
            decky_plugin.logger.error(f"Error starting/stopping server: {e}")
            raise e

    async def get_port(self) -> int:
        return settings.getSetting("PORT", 5158)

    async def set_port(self, port: int) -> int:
        settings.setSetting("PORT", int(port))
        settings.commit()
        return port

    async def get_error(self) -> str | None:
        return self.error

    def set_error(self, error: str) -> None:
        self.error = error

    async def get_accepted_warning(self) -> bool:
        return settings.getSetting("ACCEPTED_WARNING", False)

    async def set_accepted_warning(self) -> None:
        decky_plugin.logger.info("Accepted warning")
        settings.setSetting("ACCEPTED_WARNING", True)
        settings.commit()

    async def get_ip_address(self):
        return socket.gethostbyname(socket.gethostname())

    async def get_status(self):
        return {
            "server_running": self.server_running,
            "ip_address": await self.get_ip_address(),
            "port": await self.get_port(),
            "accepted_warning": await self.get_accepted_warning(),
            "error": await self.get_error(),
        }

    # Asyncio-compatible long-running code, executed in a task when the plugin is loaded
    async def _main(self):
        try:
            if settings.getSetting("PORT") is None:
                await self.set_port(5158)

            decky_plugin.logger.info("deck-screenshot-explorer: loading plugin...")
            loop = asyncio.get_event_loop()
            self._watchdog_task = loop.create_task(self._watchdog())
            decky_plugin.logger.info("deck-screenshot-explorer: plugin loaded")
        except Exception as e:
            decky_plugin.logger.error(f"Error loading plugin: {e}")
            raise e

    # Function called first during the unload process, utilize this to handle your plugin being removed
    async def _unload(self):
        decky_plugin.logger.info("deck-screenshot-explorer: unloading plugin...")
        await self.start_server(False)
        if self._watchdog_task:
            self._watchdog_task.cancel()
        decky_plugin.logger.info("deck-screenshot-explorer: plugin unloaded")
