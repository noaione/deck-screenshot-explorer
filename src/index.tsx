import {
  ButtonItem,
  definePlugin,
  DialogBody,
  DialogButton,
  DialogFooter,
  DialogHeader,
  Field,
  ModalRoot,
  PanelSection,
  PanelSectionRow,
  ServerAPI,
  showModal,
  staticClasses,
  TextField,
  ToggleField,
} from "decky-frontend-lib";
import { ChangeEvent, useEffect, useState, VFC } from "react";
import { FaImages } from "react-icons/fa";
import { IoMdWarning } from "react-icons/io";

// interface AddMethodArgs {
//   left: number;
//   right: number;
// }
interface AppState {
  server_running: boolean;
  ip_address: string;
  port: number;
  accepted_warning: boolean;
  error?: string | null;
}

const Content: VFC<{ serverAPI: ServerAPI }> = ({serverAPI}) => {
  const [state, setState] = useState<AppState>({
    server_running: false,
    ip_address: "127.0.0.1",
    port: 5158,
    accepted_warning: false,
    error: null,
  });
  const [loading, setLoading] = useState(false);
  const [killing, setKilling] = useState(false);

  const getServerStatus = async () => {
    const callState = await serverAPI.callPluginMethod<undefined, AppState>("get_status", undefined);
    if (callState.success) {
      setState(callState.result);
    }
  };

  const setLastError = async () => {
    const callState = await serverAPI.callPluginMethod<undefined, string | null | undefined>("get_error", undefined);
    if (callState.success) {
      setState((prevState) => ({ ...prevState, error: callState.result }));
    }
  }

  const forceKillServer = async () => {
    setKilling(true);
    try {
      await serverAPI.callPluginMethod<undefined, undefined>("force_kill", undefined);
    } catch (_error) {
      console.error(_error);
    } finally {
      setKilling(false);
    }
  }

  const toggleServer = async (checked: boolean) => {
    setState((prevState) => ({ ...prevState, server_running: checked, error: null }));

    if (state.accepted_warning) {
      setLoading(true);
      const callState = await serverAPI.callPluginMethod<{ enable: Boolean }, boolean>("start_server", {
        enable: checked,
      });

      if (callState.success) {
        setState((prevState) => ({ ...prevState, server_running: checked, accepted_warning: true }));
        await setLastError();
      }
      setLoading(false);

      return;
    }

    const onCancel = () => {
      setState((prevState) => ({ ...prevState, server_running: false }));
    }

    const onConfirm = async () => {
      await serverAPI.callPluginMethod<undefined, undefined>("set_accepted_warning", undefined);
      setState((prevState) => ({ ...prevState, accepted_warning: true }));
      await toggleServer(checked);
    }

    showModal(<WarningModal onCancel={onCancel} onConfirm={onConfirm} />, window);
  }

  useEffect(() => {
    getServerStatus();
    const timer = setInterval(getServerStatus, 5000);
    return () => clearInterval(timer);
  }, []);

  return (
    <>
      <PanelSection>
        <PanelSectionRow>
          <ToggleField checked={state.server_running} onChange={toggleServer} label="Enable Server" disabled={loading} />
          {state.error ? <div>{state.error}</div> : null}
        </PanelSectionRow>
      </PanelSection>
      <PanelSectionRow>
        <ButtonItem
          disabled={state.server_running || loading}
          onClick={() => showModal(
            <SettingsPage
              port={state.port}
              handleSubmit={async (port) => {
                const callState = await serverAPI.callPluginMethod<{ port: number }, number>("set_port", { port });
                if (callState.success) {
                  setState((prevState) => ({ ...prevState, port: callState.result }));
                }
              }}
            />,
            window
          )}
        >
          Settings
        </ButtonItem>
      </PanelSectionRow>
      <PanelSectionRow>
        <ButtonItem
          disabled={loading || killing}
          onClick={forceKillServer}
        >
          Force Kill
        </ButtonItem>
      </PanelSectionRow>
      <PanelSection>
        <PanelSectionRow>
          <Field
            inlineWrap="shift-children-below"
            label="Server Address"
            bottomSeparator="none"
          >
            https://steamdeck:{state.port}
          </Field>
          <Field inlineWrap="shift-children-below">
            https://{state.ip_address}:{state.port}
          </Field>
        </PanelSectionRow>
      </PanelSection>
    </>
  );
};

const WarningModal = ({
  closeModal, onCancel, onConfirm
}: {
  closeModal?: () => void;
  onCancel: () => void;
  onConfirm: () => Promise<void>;
}) => {
  const handleCancel = () => {
    onCancel();
    closeModal?.();
  };

  const handleConfirm = async () => {
    await onConfirm();
    closeModal?.();
  };

  return (
    <ModalRoot closeModal={handleCancel}>
      <DialogHeader>Warning</DialogHeader>
      <DialogBody>
        <p>Do not run this plugin on untrusted network since this expose a part of your Steam Deck to the network.</p>
        <p>
          Although the exposed part is limited to only your screenshot folder and some extra user metadata,
          you should still be careful to run this on a public network.
        </p>
      </DialogBody>
      <DialogFooter>
        <DialogButton onClick={handleConfirm}>I understand</DialogButton>
      </DialogFooter>
    </ModalRoot>
  )
};

const SettingsPage: VFC<{
  closeModal?: () => void;
  port: number;
  handleSubmit: (port: number) => Promise<void>;
}> = ({
  closeModal,
  port,
  handleSubmit
}) => {
  const [statePort, setStatePort] = useState(port);
  const [showPortError, setShowPortError] = useState(false);

  const handlePortChange = (e: ChangeEvent<HTMLInputElement>) => {
    if (isNaN(parseInt(e.currentTarget.value))) {
      return;
    }
    setShowPortError(Number(parseInt(e.currentTarget.value)) < 1024);
    setStatePort(parseInt(e.currentTarget.value));
  };

  const handleClose = () => {
    // check port is a number between 1024-65535 before closing
    if (statePort >= 1024 && statePort <= 65535) {
      handleSubmit(statePort);
      closeModal?.();
    } else {
      setShowPortError(true);
    };
  };

  return (
    <ModalRoot onCancel={handleClose}>
      <DialogHeader>Settings</DialogHeader>
      <DialogBody>
        <Field label="Port" icon={showPortError ? <IoMdWarning size={20} color="red"/> : null}>
          <TextField
            description="Must be between 1024 and 65535"
            style={{
              boxSizing: "border-box",
              width: 160,
              height: 40,
              border: showPortError ? '1px red solid' : undefined
            }}
            value={String(statePort)}
            defaultValue={String(port)}
            onChange={handlePortChange}
          />
        </Field>
      </DialogBody>
    </ModalRoot>
  );
};

export default definePlugin((serverApi: ServerAPI) => {
  return {
    title: <div className={staticClasses.Title}>Screenshot Explorer</div>,
    content: <Content serverAPI={serverApi} />,
    icon: <FaImages />,
  };
});
