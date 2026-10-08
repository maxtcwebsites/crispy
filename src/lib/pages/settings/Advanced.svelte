<script lang="ts">
  import FileText from '@lucide/svelte/icons/file-text';
  import Power from '@lucide/svelte/icons/power';
  import { api } from '../../api';
  import { store } from '../../store.svelte';
  import { prettyPath } from '../../util';
  import Button from '../../components/Button.svelte';
  import Select from '../../components/Select.svelte';
  import SettingRow from '../../components/SettingRow.svelte';
  import SettingsGroup from '../../components/SettingsGroup.svelte';
  import Toggle from '../../components/Toggle.svelte';

  const s = $derived(store.settings!);

  async function quit() {
    const ok = await store.ask({
      title: 'Quit Crispy?',
      body: 'Keyboard, mouse and clipboard sharing stop on this computer until you open it again.',
      confirm: 'Quit',
    });
    if (ok) await store.run(() => api.quitApp());
  }
</script>

<SettingsGroup title="Diagnostics">
  <SettingRow label="Log detail" description="More detail helps when reporting a problem. Info is right for everyday use." id="adv-log">
    <Select
      id="adv-log"
      bind:value={s.advanced.logLevel}
      label="Log detail"
      width={150}
      options={[
        { value: 'error', label: 'Errors only', hint: 'Just the things that broke' },
        { value: 'warn', label: 'Warnings', hint: 'Errors and anything unusual' },
        { value: 'info', label: 'Info', hint: 'Recommended' },
        { value: 'debug', label: 'Debug', hint: 'Connection and switching details' },
        { value: 'trace', label: 'Trace', hint: 'Everything — large log files' },
      ]}
    />
  </SettingRow>
  <SettingRow label="Log files" description={store.info?.logDir ? prettyPath(store.info.logDir) : undefined} id="adv-open-logs">
    <Button size="sm" icon={FileText} id="adv-open-logs" onclick={() => store.run(() => api.openLogs())}>Open logs</Button>
  </SettingRow>
  <SettingRow label="Show statistics" description="Events per second, data sent and received, and uptime — in the sidebar." id="adv-stats">
    <Toggle id="adv-stats" bind:checked={s.advanced.showStats} />
  </SettingRow>
</SettingsGroup>

<SettingsGroup title="Crispy">
  <SettingRow label="Quit Crispy" description="Stops sharing on this computer." id="adv-quit">
    <Button size="sm" variant="secondary" icon={Power} id="adv-quit" onclick={quit}>Quit</Button>
  </SettingRow>
</SettingsGroup>
