(() => {
  'use strict';
  const status = document.getElementById('status');
  const install = document.getElementById('install');
  const open = document.getElementById('open');
  const retry = document.getElementById('retry');
  const invoke = (command) => window.__TAURI__.core.invoke(command);
  let leaving = false;

  async function check() {
    install.hidden = true;
    retry.hidden = true;
    status.textContent = 'Checking for updates…';
    try {
      const info = await invoke('check_app_update');
      if (leaving) return;
      if (info.version) {
        status.textContent = `Version ${info.version} is available. Installed version: ${info.current_version}.`;
        install.hidden = false;
      } else {
        status.textContent = `Version ${info.current_version} is up to date.`;
      }
    } catch (error) {
      if (!leaving) status.textContent = String(error);
    } finally {
      if (!leaving) retry.hidden = false;
    }
  }

  open.addEventListener('click', async () => {
    if (leaving) return;
    leaving = true;
    open.disabled = true;
    install.disabled = true;
    retry.disabled = true;
    try {
      await invoke('open_application');
    } catch (error) {
      leaving = false;
      open.disabled = false;
      install.disabled = false;
      retry.disabled = false;
      status.textContent = String(error);
    }
  });

  install.addEventListener('click', async () => {
    if (leaving || install.disabled) return;
    install.disabled = true;
    open.disabled = true;
    retry.disabled = true;
    status.textContent = 'Downloading and verifying the update. The installer will restart the application.';
    try {
      await invoke('install_app_update');
    } catch (error) {
      status.textContent = String(error);
      install.hidden = true;
      install.disabled = false;
      open.disabled = false;
      retry.disabled = false;
      retry.hidden = false;
    }
  });

  retry.addEventListener('click', check);
  check();
})();
