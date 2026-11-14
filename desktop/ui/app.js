// SnapIntel's window.
//
// Everything shown here comes from the Rust side through `invoke`. Text that originates on Snapchat
// (names, bios, captions) is only ever added to the page as text, never as HTML, and only https
// addresses are used for images, video and links.
'use strict';

const tauri = window.__TAURI__;
const invoke = (command, args) => tauri.core.invoke(command, args);

const $main = document.getElementById('main');
const $topbar = document.getElementById('topbar');
const $layer = document.getElementById('layer');
const $toasts = document.getElementById('toasts');

const state = {
  settings: null,
  view: 'home', // home | loading | profile | error
  query: '',
  report: null,
  data: null,
  error: null,
  tab: 'overview',
  sort: 'newest',
  zone: 'local',
  selection: new Map(), // item key -> what to download
  job: null,
  verify: null,
  looks: null, // earlier Bitmoji looks, once asked for
};

// ---------------------------------------------------------------------------------------------
// Small building blocks

const ICONS = {
  search: '<circle cx="11" cy="11" r="7"/><path d="m20 20-3.6-3.6"/>',
  download: '<path d="M12 4v11"/><path d="m7 11 5 5 5-5"/><path d="M5 20h14"/>',
  x: '<path d="M6 6l12 12M18 6 6 18"/>',
  left: '<path d="m15 5-7 7 7 7"/>',
  right: '<path d="m9 5 7 7-7 7"/>',
  down: '<path d="m6 9 6 6 6-6"/>',
  play: '<path d="M8 5.2v13.6L19 12z"/>',
  external: '<path d="M14 5h5v5"/><path d="M19 5 10.5 13.5"/><path d="M18 14v4a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h4"/>',
  folder: '<path d="M3.5 7.5a2 2 0 0 1 2-2h3.7l2 2.2h7.3a2 2 0 0 1 2 2V17a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z"/>',
  check: '<path d="m5 12.5 4.5 4.5L19 7.5"/>',
  sliders: '<path d="M4 7h9M19 7h1M4 17h3M13 17h7"/><circle cx="16" cy="7" r="2.6"/><circle cx="10" cy="17" r="2.6"/>',
  eye: '<path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12z"/><circle cx="12" cy="12" r="2.8"/>',
  message: '<path d="M4 6.5a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2V15a2 2 0 0 1-2 2h-6.5L7 20.5V17H6a2 2 0 0 1-2-2z"/>',
  share: '<path d="M12 15V4"/><path d="m8 8 4-4 4 4"/><path d="M5 13v5a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-5"/>',
  clock: '<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>',
  calendar: '<rect x="4" y="5.5" width="16" height="14.5" rx="2"/><path d="M4 10h16M8.5 3.5v4M15.5 3.5v4"/>',
  image: '<rect x="3.5" y="4.5" width="17" height="15" rx="2"/><circle cx="9" cy="10" r="1.6"/><path d="m5 18 5-5 3.5 3.5L16 14l4 4"/>',
  video: '<rect x="3.5" y="6" width="12.5" height="12" rx="2"/><path d="m16 10.5 4.5-2.5v8L16 13.5"/>',
  layers: '<path d="m12 4 8.5 4.5L12 13 3.5 8.5z"/><path d="m3.5 12.5 8.5 4.5 8.5-4.5"/><path d="m3.5 16.5 8.5 4.5 8.5-4.5"/>',
  sparkles: '<path d="M11 4l1.7 4.6L17.5 10l-4.8 1.4L11 16l-1.7-4.6L4.5 10l4.8-1.4z"/><path d="M18 14.5l.8 2 2 .8-2 .8-.8 2-.8-2-2-.8 2-.8z"/>',
  chart: '<path d="M5 20V11M12 20V4M19 20v-6"/>',
  copy: '<rect x="8.5" y="8.5" width="11" height="11" rx="2"/><path d="M15.5 8.5V6.5a2 2 0 0 0-2-2h-7a2 2 0 0 0-2 2v7a2 2 0 0 0 2 2h2"/>',
  alert: '<path d="M12 4 2.8 19.5h18.4z"/><path d="M12 10v4.5M12 17.2v.3"/>',
  info: '<circle cx="12" cy="12" r="8.5"/><path d="M12 11v5.5M12 7.6v.3"/>',
  refresh: '<path d="M19.5 12a7.5 7.5 0 1 1-2.2-5.3"/><path d="M19.5 4.5v4h-4"/>',
  music: '<path d="M9 18V6l10-2v12"/><circle cx="6.5" cy="18" r="2.5"/><circle cx="16.5" cy="16" r="2.5"/>',
  hash: '<path d="M9.5 4 8 20M16 4l-1.5 16M4.5 9h15.5M4 15h15.5"/>',
  user: '<circle cx="12" cy="8.5" r="3.8"/><path d="M4.5 20a7.5 7.5 0 0 1 15 0"/>',
  users: '<circle cx="9" cy="9" r="3.4"/><path d="M2.8 19.5a6.2 6.2 0 0 1 12.4 0"/><path d="M15.5 5.8a3.4 3.4 0 0 1 0 6.4M17.5 14.6a6.2 6.2 0 0 1 3.7 4.9"/>',
  lens: '<circle cx="12" cy="12" r="8.5"/><circle cx="12" cy="12" r="3.6"/><path d="M12 3.5v4.9M12 15.6v4.9"/>',
  shield: '<path d="M12 3.5 5 6v5.5c0 4.3 2.9 7.4 7 9 4.1-1.6 7-4.7 7-9V6z"/><path d="m9 12 2.2 2.2L15 10.4"/>',
  qr: '<rect x="4" y="4" width="6" height="6" rx="1"/><rect x="14" y="4" width="6" height="6" rx="1"/><rect x="4" y="14" width="6" height="6" rx="1"/><path d="M14 14h2.5v2.5H14zM17.5 17.5H20V20h-2.5zM14 19v1M19.5 14v1.5"/>',
  file: '<path d="M7.5 3.5h6.5l4 4V19a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 6 19V5a1.5 1.5 0 0 1 1.5-1.5z"/><path d="M14 3.5v4h4"/>',
  table: '<rect x="4" y="5" width="16" height="14" rx="2"/><path d="M4 10h16M4 14.5h16M10 5v14"/>',
  link: '<path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1"/><path d="M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"/>',
  bolt: '<path d="M13 3 5 13.5h6L10 21l8.5-11H12z"/>',
  heart: '<path d="M12 20s-7.5-4.6-7.5-10A4.3 4.3 0 0 1 12 7.3 4.3 4.3 0 0 1 19.5 10c0 5.4-7.5 10-7.5 10z"/>',
  text: '<path d="M5 6h14M5 10h14M5 14h9M5 18h11"/>',
  verified: '<path d="M12 2.8l2.3 1.7 2.8-.2 1 2.6 2.4 1.5-.7 2.7.7 2.7-2.4 1.5-1 2.6-2.8-.2L12 21.2l-2.3-1.7-2.8.2-1-2.6-2.4-1.5.7-2.7-.7-2.7 2.4-1.5 1-2.6 2.8.2z"/><path d="m9 12 2.2 2.2L15 10.4" fill="none" stroke="var(--surface)" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"/>',
  globe: '<circle cx="12" cy="12" r="8.5"/><path d="M3.5 12h17M12 3.5c2.6 2.8 2.6 14.2 0 17M12 3.5c-2.6 2.8-2.6 14.2 0 17"/>',
  home: '<path d="M4 11 12 4l8 7"/><path d="M6 10v9h12v-9"/>',
};
const FILLED = new Set(['play', 'verified', 'bolt']);
const SVG = 'http://www.w3.org/2000/svg';

function icon(name, extra = '') {
  const svg = document.createElementNS(SVG, 'svg');
  svg.setAttribute('viewBox', '0 0 24 24');
  svg.setAttribute('class', `icon${FILLED.has(name) ? ' fill' : ''}${extra ? ` ${extra}` : ''}`);
  svg.setAttribute('aria-hidden', 'true');
  svg.innerHTML = ICONS[name] || '';
  return svg;
}

function mark(extra = '') {
  const svg = document.createElementNS(SVG, 'svg');
  svg.setAttribute('viewBox', '0 0 1024 1024');
  svg.setAttribute('class', extra);
  svg.setAttribute('aria-hidden', 'true');
  svg.innerHTML =
    '<rect x="72" y="72" width="880" height="880" rx="212" fill="#17171c"/>' +
    '<circle cx="456" cy="456" r="190" fill="none" stroke="#ffd60a" stroke-width="78"/>' +
    '<path d="M604 604 752 752" fill="none" stroke="#ffd60a" stroke-width="92" stroke-linecap="round"/>' +
    '<circle cx="456" cy="456" r="66" fill="#ffd60a"/>';
  return svg;
}

function append(element, children) {
  for (const child of children.flat(Infinity)) {
    if (child == null || child === false || child === '') continue;
    element.append(child instanceof Node ? child : document.createTextNode(String(child)));
  }
  return element;
}

/** Creates an element. `text` sets its text, `vars` sets CSS variables, `on…` adds listeners. */
function h(tag, props, ...children) {
  const element = document.createElement(tag);
  for (const [key, value] of Object.entries(props || {})) {
    if (value == null || value === false) continue;
    if (key === 'class') element.className = value;
    else if (key === 'text') element.textContent = value;
    else if (key === 'vars') for (const [name, v] of Object.entries(value)) element.style.setProperty(name, v);
    else if (key.startsWith('on')) element.addEventListener(key.slice(2), value);
    else element.setAttribute(key, value === true ? '' : value);
  }
  return append(element, children);
}

const safe = (url) => (typeof url === 'string' && /^https:\/\//i.test(url) ? url : null);

function image(src, alt = '', cls = '') {
  const url = safe(src);
  if (!url) return null;
  const element = h('img', { src: url, alt, class: cls, loading: 'lazy', decoding: 'async', referrerpolicy: 'no-referrer', draggable: 'false' });
  element.addEventListener('error', () => element.remove(), { once: true });
  return element;
}

function avatar(label, src, size = '') {
  return h('span', { class: `avatar ${size}`.trim() }, (label || '?').trim().charAt(0), image(src));
}

function button(label, { icon: iconName, kind = '', onclick, title, disabled, type = 'button' } = {}) {
  return h('button', { class: `btn ${kind}`.trim(), type, onclick, title, 'aria-label': label ? null : title, disabled }, iconName ? icon(iconName) : null, label);
}

const compactNumber = new Intl.NumberFormat('en', { notation: 'compact', maximumFractionDigits: 1 });
const fullNumber = new Intl.NumberFormat('en');
const dayFormat = new Intl.DateTimeFormat(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
const momentFormat = new Intl.DateTimeFormat(undefined, { year: 'numeric', month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit' });
const monthFormat = new Intl.DateTimeFormat(undefined, { month: 'short' });

const compact = (n) => (n == null ? '–' : compactNumber.format(n));
const plural = (n, one, many = `${one}s`) => `${fullNumber.format(n)} ${n === 1 ? one : many}`;
const parseDate = (iso) => {
  const date = iso ? new Date(iso) : null;
  return date && !Number.isNaN(date.getTime()) ? date : null;
};
const day = (iso) => (parseDate(iso) ? dayFormat.format(parseDate(iso)) : null);
const moment = (iso) => (parseDate(iso) ? momentFormat.format(parseDate(iso)) : null);

function length(seconds) {
  if (seconds == null) return null;
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const rest = String(Math.floor(seconds % 60)).padStart(2, '0');
  return hours ? `${hours}:${String(minutes).padStart(2, '0')}:${rest}` : `${minutes}:${rest}`;
}

function size(bytes) {
  if (!bytes) return '0 MB';
  const units = ['B', 'KB', 'MB', 'GB'];
  const index = Math.min(units.length - 1, Math.floor(Math.log(bytes) / Math.log(1024)));
  return `${(bytes / 1024 ** index).toFixed(index > 1 ? 1 : 0)} ${units[index]}`;
}

function message(error) {
  if (error && typeof error === 'object') return { kind: error.kind || 'other', message: error.message || 'Something went wrong' };
  return { kind: 'other', message: String(error || 'Something went wrong') };
}

function toast(text, { kind = 'info', action, duration = 4600 } = {}) {
  const element = h(
    'div',
    { class: `toast ${kind === 'error' ? 'bad' : kind === 'success' ? 'good' : ''}`.trim() },
    icon(kind === 'error' ? 'alert' : kind === 'success' ? 'check' : 'info'),
    h('span', { text }),
    action ? h('button', { type: 'button', onclick: () => { element.remove(); action.run(); } }, action.label) : null,
  );
  $toasts.append(element);
  setTimeout(() => element.remove(), duration);
}

async function copy(text, what = 'Copied') {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const area = h('textarea', { class: 'selectable' });
    area.value = text;
    document.body.append(area);
    area.select();
    document.execCommand('copy');
    area.remove();
  }
  toast(what, { kind: 'success', duration: 1800 });
}

const openLink = (url) => invoke('open_link', { url }).catch((error) => toast(message(error).message, { kind: 'error' }));
const showInFolder = (path) => invoke('show_in_folder', { path }).catch((error) => toast(message(error).message, { kind: 'error' }));

// ---------------------------------------------------------------------------------------------
// Menus, panels and the keyboard

let menu = null;
function closeMenu() {
  if (!menu) return;
  menu.remove();
  menu = null;
  document.removeEventListener('pointerdown', menuOutside, true);
}
function menuOutside(event) {
  if (menu && !menu.contains(event.target)) closeMenu();
}
function openMenu(anchor, items) {
  closeMenu();
  menu = h(
    'div',
    { class: 'menu', role: 'menu' },
    items.map((item) => h('button', { type: 'button', role: 'menuitem', onclick: () => { closeMenu(); item.run(); } }, item.icon ? icon(item.icon) : null, item.label, item.hint ? h('small', { text: item.hint }) : null)),
  );
  $layer.append(menu);
  const box = anchor.getBoundingClientRect();
  menu.style.top = `${Math.min(box.bottom + 6, innerHeight - menu.offsetHeight - 8)}px`;
  menu.style.left = `${Math.max(8, Math.min(box.right - menu.offsetWidth, innerWidth - menu.offsetWidth - 8))}px`;
  setTimeout(() => document.addEventListener('pointerdown', menuOutside, true));
  menu.querySelector('button')?.focus();
}

let panel = null; // the settings dialog or the downloads drawer
function closePanel() {
  if (!panel) return;
  panel.backdrop.remove();
  panel.element.remove();
  panel = null;
}
function openPanel(kind, element) {
  closePanel();
  closeMenu();
  const backdrop = h('div', { class: 'backdrop', onclick: closePanel });
  $layer.append(backdrop, element);
  panel = { kind, element, backdrop };
  // Focus moves into the dialog so Tab stays inside it and Esc closes it.
  element.setAttribute('tabindex', '-1');
  element.focus();
}

function panelHead(title, iconName) {
  return h('div', { class: 'modal-head' }, icon(iconName, 'lg'), h('h2', { text: title }), h('button', { class: 'btn ghost icon-only', type: 'button', title: 'Close (Esc)', 'aria-label': 'Close', onclick: closePanel }, icon('x')));
}

function focusSearch() {
  const form = state.view === 'home' ? $main.querySelector('.search') : $topbar.querySelector('.search');
  const input = form?.querySelector('input');
  input?.focus();
  input?.select();
}

document.addEventListener('keydown', (event) => {
  if (event.key === 'Escape') {
    if (menu) closeMenu();
    else if (panel) closePanel();
    else if (viewer) closeViewer();
    else if (state.selection.size) clearSelection();
    else return;
    event.preventDefault();
    return;
  }
  const typing = /^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement?.tagName || '');
  if (((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') || (event.key === '/' && !typing && !viewer && !panel)) {
    event.preventDefault();
    if (viewer) closeViewer();
    closePanel();
    focusSearch();
  }
});

// The browser's own menu (Back, Reload, Print) has no place in an app; keep it for text fields.
document.addEventListener('contextmenu', (event) => {
  if (!event.target.closest('input, textarea, .selectable')) event.preventDefault();
});

// ---------------------------------------------------------------------------------------------
// Settings

function applyTheme() {
  const theme = state.settings?.theme;
  if (theme === 'light' || theme === 'dark') document.documentElement.dataset.theme = theme;
  else delete document.documentElement.dataset.theme;
}

async function loadSettings() {
  try {
    state.settings = await invoke('get_settings');
  } catch (error) {
    toast(message(error).message, { kind: 'error' });
  }
  applyTheme();
}

async function saveSettings(change) {
  try {
    state.settings = await invoke('save_settings', { settings: { ...state.settings, ...change } });
    applyTheme();
  } catch (error) {
    toast(message(error).message, { kind: 'error' });
  }
  return state.settings;
}

const LANGUAGES = [
  ['', 'Let Snapchat choose'],
  ['en-US', 'English'],
  ['ar', 'العربية'],
  ['fr-FR', 'Français'],
  ['de-DE', 'Deutsch'],
  ['es', 'Español'],
  ['pt-BR', 'Português'],
  ['it-IT', 'Italiano'],
  ['tr-TR', 'Türkçe'],
  ['hi-IN', 'हिन्दी'],
  ['ja-JP', '日本語'],
];

function settingRow(title, description, control) {
  return h('div', { class: 'setting' }, h('div', { class: 'label' }, h('b', { text: title }), description ? h('span', { text: description }) : null), control);
}

function toggle(on, change, label) {
  const element = h('button', { class: 'switch', type: 'button', role: 'switch', 'aria-checked': String(on), 'aria-label': label });
  element.addEventListener('click', () => {
    const next = element.getAttribute('aria-checked') !== 'true';
    element.setAttribute('aria-checked', String(next));
    change(next);
  });
  return element;
}

function choice(options, value, change, label) {
  const element = h('select', { 'aria-label': label }, options.map(([key, text]) => h('option', { value: key, selected: key === value, text })));
  element.addEventListener('change', () => change(element.value));
  return element;
}

function openSettings(focusCheck = false) {
  const settings = state.settings;
  if (!settings) return;
  const folder = h('span', { class: 'selectable', text: settings.download_root });
  const reset = h('button', { class: 'btn sm ghost', type: 'button', text: 'Reset' });
  reset.hidden = !settings.download_dir;
  reset.addEventListener('click', async () => {
    const saved = await saveSettings({ download_dir: null });
    folder.textContent = saved.download_root;
    reset.hidden = true;
  });
  const change = button('Change…', {
    kind: 'sm',
    onclick: async () => {
      const picked = await invoke('pick_folder').catch(() => null);
      if (!picked) return;
      const saved = await saveSettings({ download_dir: picked });
      folder.textContent = saved.download_root;
      reset.hidden = false;
    },
  });
  const number = (value, min, max, key, label) => {
    const input = h('input', { type: 'number', min, max, value, 'aria-label': label });
    input.addEventListener('change', async () => {
      const saved = await saveSettings({ [key]: Math.max(min, Math.min(max, Number(input.value) || min)) });
      input.value = saved[key];
    });
    return input;
  };
  const checkResult = h('div');
  const checkButton = button('Check now', { icon: 'shield', kind: 'sm', onclick: () => runSelfCheck(checkButton, checkResult) });

  const body = h(
    'div',
    { class: 'modal-body' },
    h('div', { class: 'group-title', text: 'Downloads' }),
    h('div', { class: 'setting' }, h('div', { class: 'label' }, h('b', { text: 'Save to' }), folder), reset, change),
    settingRow('A folder for each account', 'Keeps every profile\'s files and their record together.', toggle(settings.folder_per_account, (on) => saveSettings({ folder_per_account: on }), 'A folder for each account')),
    settingRow('Downloads at once', 'How many files are fetched at the same time (1 to 16).', number(settings.concurrency, 1, 16, 'concurrency', 'Downloads at once')),
    h('div', { class: 'group-title', text: 'Snapchat' }),
    settingRow('Page language', 'The language Snapchat writes page titles and descriptions in.', choice(LANGUAGES, settings.locale, (value) => saveSettings({ locale: value }), 'Page language')),
    settingRow('Give up after', 'Seconds to wait for Snapchat before reporting a problem (5 to 120).', number(settings.timeout_secs, 5, 120, 'timeout_secs', 'Seconds to wait')),
    h('div', { class: 'group-title', text: 'Appearance' }),
    settingRow('Theme', null, choice([['system', 'Match the system'], ['light', 'Light'], ['dark', 'Dark']], settings.theme, (value) => saveSettings({ theme: value }), 'Theme')),
    h('div', { class: 'group-title', text: 'Privacy' }),
    settingRow('Remember recent lookups', 'Shows the last profiles you opened on the start screen. Kept only on this computer.', toggle(settings.remember_recent, async (on) => {
      await saveSettings({ remember_recent: on });
      if (state.view === 'home') render();
    }, 'Remember recent lookups')),
    h('div', { class: 'group-title', text: 'Compatibility' }),
    settingRow('Check Snapchat\'s page', 'SnapIntel reads the profile pages on snapchat.com. If Snapchat changes them, some details can go missing; this check tells you.', checkButton),
    checkResult,
    h('p', { class: 'about selectable' }, `SnapIntel ${settings.version} · AGPL-3.0 · based on SnapIntel by Kr0wZ. It shows only what a profile publishes on snapchat.com.`),
  );
  openPanel('settings', h('div', { class: 'modal', role: 'dialog', 'aria-modal': 'true', 'aria-label': 'Settings' }, panelHead('Settings', 'sliders'), body));
  if (focusCheck) {
    checkButton.scrollIntoView({ block: 'center' });
    runSelfCheck(checkButton, checkResult);
  }
}

async function runSelfCheck(trigger, target) {
  trigger.disabled = true;
  target.replaceChildren(h('div', { class: 'note' }, h('span', { class: 'spinner' }), 'Loading two of Snap\'s own profiles to compare…'));
  try {
    const report = await invoke('self_check');
    const fields = report.accounts.flatMap((account) => account.fields.map((field) => ({ ...field, account: account.account })));
    const unreadable = report.accounts.filter((account) => !account.layout);
    const problems = fields.filter((field) => field.status === 'missing');
    const row = (field) =>
      h('div', { class: 'check-row' }, h('span', { class: `pill ${field.status === 'ok' ? 'ok' : field.status === 'missing' ? 'bad' : ''}`.trim(), text: field.status }), h('code', { text: field.name }), h('span', { text: field.detail, title: `${field.account}: ${field.path}` }));
    const list = h('div', { class: 'check-list selectable' }, (problems.length ? problems : fields).map(row));
    const summary = report.healthy
      ? h('div', { class: 'verify-result good' }, h('b', { text: 'Everything SnapIntel reads is where it should be.' }), `${report.ok} details confirmed, ${report.empty + report.skipped} not filled in on the test profiles.`)
      : h('div', { class: 'verify-result bad' }, h('b', { text: unreadable.length ? 'Snapchat couldn\'t be reached for the check.' : 'Snapchat has changed its page.' }), unreadable.length ? unreadable.map((account) => `${account.account}: ${account.error}`).join(' ') : `${problems.length} ${problems.length === 1 ? 'detail is' : 'details are'} no longer where SnapIntel looks. An update is needed to read ${problems.length === 1 ? 'it' : 'them'} again.`);
    target.replaceChildren(summary, unreadable.length && !fields.length ? '' : list);
  } catch (error) {
    target.replaceChildren(h('div', { class: 'verify-result bad', text: message(error).message }));
  }
  trigger.disabled = false;
}

// ---------------------------------------------------------------------------------------------
// From a report to things that can be shown, opened and downloaded

function derive(report) {
  const user = report.profile.username || report.requested_username;
  const file = (url, type, category, uploaded) => (safe(url) ? { url, ext: type === 'image' ? 'jpg' : 'mp4', category, uploaded_at: uploaded || null } : null);
  const fromSnap = (snap, section, more) => ({
    key: `${section}:${snap.id}`,
    section,
    type: snap.media_type,
    media: snap.media_url,
    thumb: snap.preview_url || (snap.media_type === 'image' ? snap.media_url : null),
    uploaded_at: snap.uploaded_at,
    transcript_url: snap.transcript_url,
    download: file(snap.media_url, snap.media_type, section, snap.uploaded_at),
    ...more,
  });
  const story = report.story.map((snap, index) => fromSnap(snap, 'story', { title: `Story · snap ${index + 1}`, position: `${index + 1} of ${report.story.length}` }));
  const highlights = report.highlights.map((highlight, index) => {
    const title = highlight.title || `Highlight ${index + 1}`;
    const snaps = highlight.snaps.map((snap, at) => fromSnap(snap, 'highlight', { title, position: `${at + 1} of ${highlight.snaps.length}` }));
    const dates = snaps.map((snap) => snap.uploaded_at).filter(Boolean).sort();
    return { key: `group:${highlight.id || index}`, title, thumb: highlight.thumbnail_url || snaps[0]?.thumb, snaps, first: dates[0] || null, last: dates[dates.length - 1] || null };
  });
  const spotlights = report.spotlights
    .map((spotlight) => {
      const snap = spotlight.snaps[0];
      if (!snap) return null;
      const uploaded = spotlight.uploaded_at || snap.uploaded_at;
      return {
        ...fromSnap(snap, 'spotlight', {}),
        key: `spotlight:${spotlight.id || snap.id}`,
        thumb: spotlight.thumbnail_url || snap.preview_url,
        title: spotlight.description || spotlight.ai_title || spotlight.name || 'Spotlight',
        uploaded_at: uploaded,
        spotlight,
        download: file(snap.media_url, snap.media_type, 'spotlight', uploaded),
      };
    })
    .filter(Boolean);
  const lenses = report.lenses.map((lens, index) => ({
    key: `lens:${index}`,
    section: 'lens',
    type: lens.preview_video_url ? 'video' : 'image',
    media: lens.preview_video_url || lens.preview_image_url,
    thumb: lens.preview_image_url || lens.icon_url,
    title: lens.name || 'Lens',
    uploaded_at: lens.updated_at,
    lens,
    download: file(lens.preview_video_url, 'video', 'lens', null),
  }));
  return { user, story, highlights, spotlights, lenses };
}

const everything = (data) => [...data.story, ...data.highlights.flatMap((group) => group.snaps), ...data.spotlights, ...data.lenses];

// ---------------------------------------------------------------------------------------------
// Looking a profile up

let lookups = 0;
async function lookup(input) {
  const query = String(input || '').trim();
  if (!query) {
    focusSearch();
    return;
  }
  closeMenu();
  closePanel();
  closeViewer();
  const ticket = ++lookups;
  Object.assign(state, { query, view: 'loading', error: null, looks: null, verify: null });
  state.selection.clear();
  render();
  try {
    const report = await invoke('lookup', { username: query });
    if (ticket !== lookups) return;
    Object.assign(state, { report, data: derive(report), tab: 'overview', sort: 'newest', view: 'profile' });
    loadSettings().then(() => {}); // the list of recent lookups has changed
  } catch (error) {
    if (ticket !== lookups) return;
    Object.assign(state, { error: message(error), view: 'error' });
  }
  render();
}

function goHome() {
  lookups += 1;
  closeViewer();
  closePanel();
  state.view = 'home';
  state.selection.clear();
  render();
  focusSearch();
}

function searchForm(big) {
  const input = h('input', {
    type: 'text',
    placeholder: big ? 'Username or profile link' : 'Look up another profile',
    'aria-label': 'Snapchat username or profile link',
    spellcheck: 'false',
    autocomplete: 'off',
    autocapitalize: 'off',
    autocorrect: 'off',
  });
  return h(
    'form',
    { class: 'search', role: 'search', novalidate: true, onsubmit: (event) => { event.preventDefault(); lookup(input.value); } },
    big ? icon('search', 'lg') : h('span', { class: 'at', text: '@' }),
    input,
    big ? h('button', { class: 'btn primary', type: 'submit', text: 'Look up' }) : h('kbd', { text: 'Ctrl K' }),
  );
}

function renderTopbar() {
  const downloads = h('button', { class: 'btn ghost icon-only dl-button', type: 'button', title: 'Downloads', 'aria-label': 'Downloads', onclick: openDownloads }, icon('download'), h('span', { class: 'ring' }), h('span', { class: 'dot' }));
  $topbar.replaceChildren(
    h('button', { class: 'brand', type: 'button', title: 'Start screen', onclick: goHome }, mark('brand-mark'), 'SnapIntel'),
    h('div', { class: 'grow' }, searchForm(false)),
    h('div', { class: 'topbar-actions' }, downloads, h('button', { class: 'btn ghost icon-only', type: 'button', title: 'Settings', 'aria-label': 'Settings', onclick: () => openSettings() }, icon('sliders'))),
  );
}

// ---------------------------------------------------------------------------------------------
// Views

function render() {
  document.body.dataset.view = state.view;
  const view = { home: renderHome, loading: renderLoading, profile: renderProfile, error: renderProblem }[state.view]();
  $main.replaceChildren(view);
  $main.scrollTop = 0;
  const top = $topbar.querySelector('.search input');
  if (top) top.value = state.view === 'profile' ? '' : state.view === 'home' ? '' : state.query;
  renderSelectionBar();
}

function renderHome() {
  const recent = state.settings?.remember_recent ? state.settings.recent || [] : [];
  const feature = (iconName, title, text) => h('div', { class: 'feature' }, icon(iconName, 'lg'), h('b', { text: title }), h('span', { text }));
  return h(
    'div',
    { class: 'home' },
    h(
      'div',
      { class: 'home-inner' },
      mark('home-mark'),
      h('h1', { text: 'Look up a Snapchat profile' }),
      h('p', { class: 'lede', text: 'See what a public profile shares on Snapchat: its story, highlights, Spotlight videos and lenses.' }),
      searchForm(true),
      h('p', { class: 'hint', text: 'Type a username or paste a profile link, then press Enter.' }),
      recent.length
        ? h(
            'div',
            { class: 'recent' },
            h('div', { class: 'recent-head' }, 'Recent', h('button', { type: 'button', text: 'Clear', onclick: async () => { await saveSettings({ recent: [] }); render(); focusSearch(); } })),
            h('div', { class: 'recent-list' }, recent.map((entry) => h('button', { class: 'person-chip', type: 'button', title: `@${entry.username}`, onclick: () => lookup(entry.username) }, avatar(entry.display_name || entry.username, entry.picture_url), h('span', { dir: 'auto', text: entry.display_name || entry.username })))),
          )
        : null,
      h(
        'div',
        { class: 'features' },
        feature('layers', 'Everything in one place', 'Profile details, story, highlights, Spotlight and lenses.'),
        feature('shield', 'Downloads you can check', 'Each file is recorded with its source, date and SHA-256.'),
        feature('table', 'Take the data with you', 'Export a profile to JSON or a CSV spreadsheet.'),
      ),
    ),
  );
}

function renderLoading() {
  const block = (style) => {
    const element = h('div', { class: 'skeleton' });
    Object.assign(element.style, style);
    return element;
  };
  return h(
    'div',
    { class: 'page', 'aria-busy': 'true', 'aria-label': `Loading ${state.query}` },
    block({ height: '250px', borderRadius: '20px' }),
    h('div', { class: 'tabs' }, [90, 70, 100, 90, 70].map((width) => block({ width: `${width}px`, height: '22px', margin: '10px 10px 12px 0' }))),
    h('div', { class: 'grid' }, Array.from({ length: 10 }, () => block({ aspectRatio: '9 / 16', borderRadius: '14px' }))),
  );
}

function renderProblem() {
  const { kind, message: detail } = state.error || {};
  const copyOf = {
    not_found: ['user', 'No profile with that name', 'Snapchat has no profile called “' + state.query + '”. Check the spelling: usernames are not the same as display names.'],
    invalid_username: ['user', 'That isn\'t a username', 'Usernames are 3 to 15 letters, numbers, dots, dashes or underscores. You can also paste a profile link.'],
    network: ['globe', 'Can\'t reach Snapchat', detail],
    timeout: ['clock', 'Snapchat is taking too long', 'It didn\'t answer in time. Check your connection and try again.'],
    http: ['alert', 'Snapchat turned the request away', `${detail}. It may be limiting requests for a while; try again in a minute.`],
    page_format: ['alert', 'Snapchat\'s page has changed', `${detail}. A compatibility check will show what moved.`],
  }[kind] || ['alert', 'Something went wrong', detail];
  return h(
    'div',
    { class: 'problem' },
    h(
      'div',
      { class: 'problem-inner' },
      h('div', { class: 'badge' }, icon(copyOf[0], 'xl')),
      h('h1', { text: copyOf[1] }),
      h('p', { class: 'selectable', dir: 'auto', text: copyOf[2] }),
      h(
        'div',
        { class: 'actions' },
        kind === 'not_found' || kind === 'invalid_username' ? button('Try another name', { kind: 'primary', icon: 'search', onclick: () => { focusSearch(); } }) : button('Try again', { kind: 'primary', icon: 'refresh', onclick: () => lookup(state.query) }),
        kind === 'page_format' ? button('Run the check', { icon: 'shield', onclick: () => openSettings(true) }) : button('Start screen', { icon: 'home', onclick: goHome }),
      ),
    ),
  );
}

// --- Profile ---------------------------------------------------------------------------------

let profileParts = null;

function tabsFor(report, data) {
  if (!report.profile.public) return [['overview', 'Overview'], ['bitmoji', 'Bitmoji']];
  return [
    ['overview', 'Overview'],
    ['story', 'Story', data.story.length],
    ['highlights', 'Highlights', data.highlights.length],
    ['spotlight', 'Spotlight', data.spotlights.length],
    ['lenses', 'Lenses', data.lenses.length],
    ['insights', 'Insights'],
  ];
}

function renderProfile() {
  const { report, data } = state;
  const profile = report.profile;
  const name = profile.display_name || profile.username;
  const files = everything(data).map((item) => item.download).filter(Boolean);
  const stat = (value, label, title) => h('div', { class: 'stat', title }, h('b', { text: value }), h('span', { text: label }));
  const stats = [
    profile.subscriber_count ? stat(compact(profile.subscriber_count), 'Subscribers', fullNumber.format(profile.subscriber_count)) : null,
    profile.public && data.story.length ? stat(String(data.story.length), 'In the story now') : null,
    profile.public ? stat(String(data.highlights.length) + (report.more_available.highlights ? '+' : ''), 'Highlights') : null,
    profile.public ? stat(String(data.spotlights.length) + (report.more_available.spotlights ? '+' : ''), 'Spotlights') : null,
    profile.public ? stat(String(data.lenses.length) + (report.more_available.lenses ? '+' : ''), 'Lenses') : null,
    profile.created_at ? stat(String(parseDate(profile.created_at).getFullYear()), 'On Snapchat since', day(profile.created_at)) : null,
  ].filter(Boolean);
  const exportButton = button('Export', { icon: 'file', onclick: () => openMenu(exportButton, [
    { label: 'JSON', hint: 'everything', icon: 'file', run: () => exportReport('json') },
    { label: 'CSV spreadsheet', hint: 'one row per item', icon: 'table', run: () => exportReport('csv') },
  ]) });
  append(exportButton, [icon('down', 'sm')]);

  const tabs = h('div', { class: 'tabs', role: 'tablist' }, tabsFor(report, data).map(([id, label, count]) => h('button', { class: 'tab', type: 'button', role: 'tab', 'data-tab': id, 'aria-selected': String(state.tab === id), onclick: () => showTab(id) }, label, count != null ? h('span', { class: 'count', text: String(count) }) : null)));
  const content = h('div', { role: 'tabpanel' });
  profileParts = { tabs, content };
  renderTab();

  return h(
    'div',
    { class: 'page' },
    h(
      'section',
      { class: 'profile-head' },
      h('div', { class: 'hero' }, image(profile.hero_image_url || profile.background_url || profile.profile_picture_url)),
      h(
        'div',
        { class: 'head-body' },
        avatar(name, profile.profile_picture_url || profile.avatar_url, 'lg'),
        h(
          'div',
          { class: 'identity' },
          h('h1', null, h('span', { class: 'selectable', dir: 'auto', text: name }), profile.badge ? h('span', { class: 'verified', title: profile.badge === 'public_figure' ? 'Public figure' : 'Creator' }, icon('verified', 'lg')) : null),
          h(
            'div',
            { class: 'handle' },
            h('span', { class: 'selectable', text: `@${profile.username}` }),
            profile.badge ? [h('i', { class: 'sep' }), profile.badge === 'public_figure' ? 'Public figure' : 'Creator'] : null,
            profile.category ? [h('i', { class: 'sep' }), profile.category] : null,
            !profile.public ? [h('i', { class: 'sep' }), h('span', { class: 'pill', text: 'Not a public profile' })] : null,
          ),
          profile.bio ? h('p', { class: 'bio selectable', dir: 'auto', text: profile.bio }) : null,
        ),
        h(
          'div',
          { class: 'head-actions' },
          files.length ? button(`Download all (${files.length})`, { kind: 'primary', icon: 'download', onclick: () => startDownload(files) }) : null,
          exportButton,
          h('button', { class: 'btn icon-only', type: 'button', title: 'Open on Snapchat', 'aria-label': 'Open on Snapchat', onclick: () => openLink(report.canonical_url || report.url) }, icon('external')),
        ),
      ),
      stats.length ? h('div', { class: 'statline' }, stats) : null,
    ),
    tabs,
    content,
  );
}

function showTab(id) {
  state.tab = id;
  for (const tab of profileParts.tabs.children) tab.setAttribute('aria-selected', String(tab.dataset.tab === id));
  renderTab();
}

function renderTab() {
  const view = { overview: tabOverview, story: tabStory, highlights: tabHighlights, spotlight: tabSpotlight, lenses: tabLenses, insights: tabInsights, bitmoji: tabBitmoji }[state.tab] || tabOverview;
  profileParts.content.replaceChildren(view());
}

function empty(iconName, title, text) {
  return h('div', { class: 'empty' }, h('div', null, icon(iconName, 'xl'), h('b', { text: title }), h('p', { text })));
}

function moreNote(what) {
  return h('div', { class: 'note' }, icon('info', 'sm'), `Snapchat's page lists the latest ${what}; the profile has more.`, h('button', { type: 'button', text: 'Open on Snapchat', onclick: () => openLink(state.report.canonical_url || state.report.url) }));
}

function toolbar(title, sub, ...actions) {
  return h('div', { class: 'toolbar' }, h('h2', { text: title }), sub ? h('span', { class: 'sub', text: sub }) : null, h('span', { class: 'spacer' }), actions);
}

function downloadAll(items) {
  const files = items.map((item) => item.download).filter(Boolean);
  return files.length ? button(`Download all (${files.length})`, { icon: 'download', kind: 'sm', onclick: () => startDownload(files) }) : null;
}

// --- Tiles and selection ---------------------------------------------------------------------

function isSelected(items) {
  return items.length > 0 && items.every((item) => state.selection.has(item.key));
}

function setSelected(items, on) {
  for (const item of items) {
    if (!item.download) continue;
    if (on) state.selection.set(item.key, item.download);
    else state.selection.delete(item.key);
  }
  renderSelectionBar();
}

function clearSelection() {
  state.selection.clear();
  for (const tile of document.querySelectorAll('.tile.selected')) {
    tile.classList.remove('selected');
    tile.querySelector('.check')?.setAttribute('aria-checked', 'false');
  }
  renderSelectionBar();
}

let selectionBar = null;
function renderSelectionBar() {
  const count = state.selection.size;
  selectionBar?.remove();
  selectionBar = null;
  $main.classList.toggle('selecting', count > 0);
  if (!count || state.view !== 'profile') return;
  selectionBar = h(
    'div',
    { class: 'selection-bar', role: 'region', 'aria-label': 'Selected items' },
    `${plural(count, 'item')} selected`,
    button('Clear', { kind: 'ghost sm', onclick: clearSelection }),
    button('Download', { kind: 'primary sm round', icon: 'download', onclick: () => { const files = [...state.selection.values()]; clearSelection(); startDownload(files); } }),
  );
  $layer.append(selectionBar);
}

/**
 * A 9:16 card. `covers` are the items its checkbox selects (one, or a highlight's snaps),
 * `open` runs when it is activated.
 */
function tile({ thumb, title, meta = [], tags = [], covers = [], open, extra, alt = '' }) {
  const selectable = covers.some((item) => item.download);
  const element = h('div', { class: 'tile', role: 'button', tabindex: '0', 'aria-label': title || alt || 'Open' });
  const check = selectable ? h('button', { class: 'check', type: 'button', role: 'checkbox', 'aria-checked': String(isSelected(covers)), 'aria-label': 'Select', title: 'Select' }, icon('check', 'sm')) : null;
  if (check) {
    element.classList.toggle('selected', isSelected(covers));
    check.addEventListener('click', (event) => {
      event.stopPropagation();
      const on = !isSelected(covers);
      setSelected(covers, on);
      check.setAttribute('aria-checked', String(on));
      element.classList.toggle('selected', on);
    });
  }
  append(element, [
    h('div', { class: 'placeholder' }, icon('image', 'xl')),
    image(thumb, alt, 'cover'),
    h('div', { class: 'shade' }),
    h('div', { class: 'top' }, check || h('span'), h('span', null, tags)),
    h('div', { class: 'play' }, h('i', null, icon('play', 'lg'))),
    extra,
    title || meta.length ? h('div', { class: 'caption' }, title ? h('div', { class: 'title', dir: 'auto', text: title }) : null, meta.length ? h('div', { class: 'meta' }, meta) : null) : null,
  ]);
  element.addEventListener('click', open);
  element.addEventListener('keydown', (event) => {
    if (event.target !== element) return;
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      open();
    }
  });
  return element;
}

const tag = (iconName, text) => h('span', { class: 'tag' }, iconName ? icon(iconName, 'sm') : null, text);
const metaItem = (iconName, text) => h('span', null, icon(iconName, 'sm'), text);

function snapTile(item, list, index, context) {
  return tile({
    thumb: item.thumb,
    title: null,
    meta: [item.uploaded_at ? h('span', { text: day(item.uploaded_at) }) : null].filter(Boolean),
    tags: [tag(item.type === 'image' ? 'image' : 'video', '')],
    covers: [item],
    open: () => openViewer(list, index, context),
    alt: item.title,
  });
}

function spotlightTile(item, list, index) {
  const s = item.spotlight;
  return tile({
    thumb: item.thumb,
    title: item.title,
    meta: [s.views != null ? metaItem('eye', compact(s.views)) : null, s.comments ? metaItem('message', compact(s.comments)) : null, item.uploaded_at ? h('span', { text: day(item.uploaded_at) }) : null].filter(Boolean),
    tags: [s.duration_seconds != null ? tag(null, length(s.duration_seconds)) : null],
    covers: [item],
    open: () => openViewer(list, index, { kicker: 'Spotlight' }),
  });
}

// --- Tabs ------------------------------------------------------------------------------------

function tabOverview() {
  const { report, data } = state;
  const profile = report.profile;
  if (!profile.public) {
    return h(
      'div',
      { class: 'columns' },
      h('div', { class: 'card' }, h('h3', null, 'About'), h('p', { text: 'This isn\'t a public profile, so Snapchat shows only its name, its Snapcode and, when it has one, its Bitmoji. Stories and Spotlight need a public profile.' })),
      snapcodeCard(profile),
    );
  }
  const fact = (iconName, label, value) => (value ? [h('dt', null, icon(iconName, 'sm'), label), h('dd', { class: 'selectable' }, value)] : null);
  const linkTo = (url) => h('button', { class: 'linkish', type: 'button', text: url.replace(/^https?:\/\/(www\.)?/, '').replace(/\/$/, ''), title: url, onclick: () => openLink(url) });
  const about = h(
    'div',
    { class: 'card' },
    h('h3', null, 'About'),
    h(
      'dl',
      { class: 'facts' },
      fact('user', 'Username', `@${profile.username}`),
      fact('layers', 'Category', [profile.category, profile.subcategory].filter((v, i, all) => v && all.indexOf(v) === i).join(' · ')),
      fact('users', 'Subscribers', profile.subscriber_count ? fullNumber.format(profile.subscriber_count) : null),
      fact('calendar', 'On Snapchat since', day(profile.created_at)),
      fact('refresh', 'Profile updated', day(profile.updated_at)),
      fact('globe', 'Website', profile.website_url ? linkTo(profile.website_url) : null),
      fact('link', 'Also on', profile.links.length ? profile.links.map((url) => h('div', null, linkTo(url))) : null),
      fact('external', 'Profile link', linkTo(report.canonical_url || report.url)),
    ),
  );
  const top = [...data.spotlights].filter((item) => item.spotlight.views != null).sort((a, b) => b.spotlight.views - a.spotlight.views).slice(0, 4);
  const related = report.related_accounts;
  return h(
    'div',
    { class: 'columns' },
    h(
      'div',
      { class: 'stack-col' },
      about,
      data.highlights.length
        ? h('div', { class: 'card' }, h('h3', null, 'Latest highlights', h('span', { class: 'spacer' }), h('button', { class: 'linkish', type: 'button', text: 'See all', onclick: () => showTab('highlights') })), h('div', { class: 'strip' }, data.highlights.slice(0, 5).map(highlightTile)))
        : null,
      related.length
        ? h(
            'div',
            { class: 'card' },
            h('h3', null, 'Related accounts'),
            h('div', { class: 'rows' }, related.map((account) => h('button', { class: 'row', type: 'button', onclick: () => lookup(account.username) }, avatar(account.display_name || account.username, account.profile_picture_url, 'md'), h('span', { class: 'text' }, h('b', { dir: 'auto', text: account.display_name || account.username }), h('span', { text: `@${account.username}` })), h('span', { class: 'trail' }, icon('right', 'sm'))))),
          )
        : null,
    ),
    h(
      'div',
      { class: 'stack-col' },
      snapcodeCard(profile),
      top.length
        ? h(
            'div',
            { class: 'card' },
            h('h3', null, 'Most viewed Spotlights'),
            h('div', { class: 'rows' }, top.map((item) => h('button', { class: 'row', type: 'button', onclick: () => openViewer(data.spotlights, data.spotlights.indexOf(item), { kicker: 'Spotlight' }) }, h('span', { class: 'thumb' }, image(item.thumb)), h('span', { class: 'text' }, h('b', { dir: 'auto', text: item.title }), h('span', { text: day(item.uploaded_at) || '' })), h('span', { class: 'trail' }, icon('eye', 'sm'), compact(item.spotlight.views))))),
          )
        : null,
    ),
  );
}

function snapcodeCard(profile) {
  if (!safe(profile.snapcode_url)) return null;
  const save = (format) => async () => {
    try {
      const path = await invoke('save_snapcode', { username: profile.username, format });
      toast(`Snapcode saved as ${format.toUpperCase()}`, { kind: 'success', action: { label: 'Show', run: () => showInFolder(path) } });
    } catch (error) {
      toast(message(error).message, { kind: 'error' });
    }
  };
  return h(
    'div',
    { class: 'card' },
    h('h3', null, 'Snapcode'),
    h('div', { class: 'snapcode' }, h('div', { class: 'code' }, image(profile.snapcode_url, `Snapcode for ${profile.username}`)), h('div', { class: 'actions' }, button('Save SVG', { kind: 'sm', icon: 'download', onclick: save('svg') }), button('Save PNG', { kind: 'sm', icon: 'download', onclick: save('png') }))),
  );
}

function tabStory() {
  const items = state.data.story;
  if (!items.length) return empty('clock', 'No story right now', 'Stories disappear after a day. When this profile has one up, its snaps appear here.');
  return h('div', null, toolbar('Story', plural(items.length, 'snap'), downloadAll(items)), h('div', { class: 'grid' }, items.map((item, index) => snapTile(item, items, index, { kicker: 'Story' }))));
}

function highlightTile(group) {
  const range = group.first ? (day(group.first) === day(group.last) ? day(group.first) : `${day(group.first)} – ${day(group.last)}`) : null;
  return tile({
    thumb: group.thumb,
    title: group.title,
    meta: [h('span', { text: plural(group.snaps.length, 'snap') }), range ? h('span', { text: range }) : null].filter(Boolean),
    tags: [tag('layers', String(group.snaps.length))],
    covers: group.snaps,
    open: () => (group.snaps.length ? openViewer(group.snaps, 0, { kicker: 'Highlight', sequence: true }) : null),
  });
}

function tabHighlights() {
  const groups = state.data.highlights;
  if (!groups.length) return empty('layers', 'No highlights', 'This profile hasn\'t pinned any highlights.');
  const snaps = groups.flatMap((group) => group.snaps);
  return h('div', null, toolbar('Highlights', `${plural(groups.length, 'highlight')} · ${plural(snaps.length, 'snap')}`, downloadAll(snaps)), state.report.more_available.highlights ? moreNote('highlights') : null, h('div', { class: 'grid wide' }, groups.map(highlightTile)));
}

function tabSpotlight() {
  const all = state.data.spotlights;
  if (!all.length) return empty('video', 'No Spotlight videos', 'This profile hasn\'t posted to Spotlight.');
  const sorted = [...all];
  if (state.sort === 'views') sorted.sort((a, b) => (b.spotlight.views ?? -1) - (a.spotlight.views ?? -1));
  else sorted.sort((a, b) => String(b.uploaded_at || '').localeCompare(String(a.uploaded_at || '')));
  const option = (id, label) => h('button', { type: 'button', 'aria-pressed': String(state.sort === id), text: label, onclick: () => { state.sort = id; renderTab(); } });
  return h(
    'div',
    null,
    toolbar('Spotlight', plural(all.length, 'video'), h('div', { class: 'segmented', role: 'group', 'aria-label': 'Sort' }, option('newest', 'Newest'), option('views', 'Most viewed')), downloadAll(all)),
    state.report.more_available.spotlights ? moreNote('Spotlights') : null,
    h('div', { class: 'grid wide' }, sorted.map((item, index) => spotlightTile(item, sorted, index))),
  );
}

function tabLenses() {
  const items = state.data.lenses;
  if (!items.length) return empty('lens', 'No lenses', 'This profile hasn\'t published any lenses.');
  return h(
    'div',
    null,
    toolbar('Lenses', plural(items.length, 'lens', 'lenses'), downloadAll(items)),
    state.report.more_available.lenses ? moreNote('lenses') : null,
    h(
      'div',
      { class: 'grid wide' },
      items.map((item, index) =>
        tile({
          thumb: item.thumb,
          title: item.title,
          meta: [item.lens.creator ? h('span', { dir: 'auto', text: item.lens.creator }) : null].filter(Boolean),
          tags: [item.lens.official ? tag('verified', 'Official') : null],
          covers: [item],
          extra: safe(item.lens.icon_url) && item.lens.preview_image_url ? h('div', { class: 'lens-icon' }, image(item.lens.icon_url)) : null,
          open: () => openViewer(items, index, { kicker: 'Lens' }),
        }),
      ),
    ),
  );
}

function tabInsights() {
  const { data, report } = state;
  const dated = [...data.story, ...data.highlights.flatMap((group) => group.snaps), ...data.spotlights].map((item) => parseDate(item.uploaded_at)).filter(Boolean);
  if (!dated.length && !data.spotlights.length) return empty('chart', 'Nothing to chart yet', 'Insights are drawn from the dated stories, highlights and Spotlights on the profile.');
  const stats = report.stats;
  const viewed = data.spotlights.filter((item) => item.spotlight.views != null);
  const metric = (value, label, iconName) => h('div', { class: 'metric' }, h('b', { text: value }), h('span', null, icon(iconName, 'sm'), label));
  const tiles = h(
    'div',
    { class: 'tiles' },
    stats.spotlight_views != null ? metric(compact(stats.spotlight_views), 'Spotlight views', 'eye') : null,
    viewed.length ? metric(compact(Math.round(stats.spotlight_views / viewed.length)), 'Views per Spotlight', 'chart') : null,
    stats.spotlight_comments != null ? metric(compact(stats.spotlight_comments), 'Comments', 'message') : null,
    stats.spotlight_shares != null ? metric(compact(stats.spotlight_shares), 'Shares', 'share') : null,
    stats.spotlight_duration_seconds ? metric(length(stats.spotlight_duration_seconds), 'Of Spotlight video', 'clock') : null,
    metric(String(stats.highlight_snaps + stats.story_snaps), 'Story and highlight snaps', 'layers'),
  );

  // When uploads happened: weekday by hour.
  const utc = state.zone === 'utc';
  const week = Array.from({ length: 7 }, () => Array(24).fill(0));
  for (const date of dated) week[((utc ? date.getUTCDay() : date.getDay()) + 6) % 7][utc ? date.getUTCHours() : date.getHours()] += 1;
  const peak = Math.max(1, ...week.flat());
  const days = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
  const heat = h(
    'div',
    { class: 'heatmap', role: 'img', 'aria-label': 'Uploads by weekday and hour' },
    h('span'),
    Array.from({ length: 24 }, (_, hour) => h('span', { class: 'hour', text: hour % 3 === 0 ? String(hour) : '' })),
    week.map((hours, d) => [h('span', { text: days[d] }), hours.map((n, hour) => h('span', { class: 'cell', 'data-n': n || null, vars: { '--v': n ? (0.18 + 0.82 * (n / peak)).toFixed(3) : 0 }, title: n ? `${days[d]} ${hour}:00 · ${plural(n, 'upload')}` : null }))]),
  );
  const zone = (id, label) => h('button', { type: 'button', 'aria-pressed': String(state.zone === id), text: label, onclick: () => { state.zone = id; renderTab(); } });
  const heatCard = h(
    'div',
    { class: 'card' },
    h('h3', null, 'Upload times', h('span', { class: 'spacer' }), h('div', { class: 'segmented', role: 'group', 'aria-label': 'Time zone' }, zone('local', 'Your time'), zone('utc', 'UTC'))),
    heat,
    h('div', { class: 'legend' }, 'Fewer', [0, 0.3, 0.55, 0.8, 1].map((v) => h('i', { vars: { '--v': v } })), 'More', h('span', { class: 'spacer' }), `From ${plural(dated.length, 'dated item')} on the page`),
  );

  // Uploads per month, for the last twelve months that have any.
  let monthCard = null;
  if (dated.length) {
    const latest = new Date(Math.max(...dated));
    const months = Array.from({ length: 12 }, (_, back) => new Date(latest.getFullYear(), latest.getMonth() - (11 - back), 1));
    const counts = months.map((month) => dated.filter((date) => date.getFullYear() === month.getFullYear() && date.getMonth() === month.getMonth()).length);
    const most = Math.max(1, ...counts);
    monthCard = h('div', { class: 'card' }, h('h3', null, 'Uploads per month'), h('div', { class: 'months' }, months.map((month, index) => h('div', { class: `month${counts[index] ? '' : ' zero'}`, title: `${monthFormat.format(month)} ${month.getFullYear()} · ${plural(counts[index], 'upload')}` }, h('i', { vars: { '--v': counts[index] / most } }), monthFormat.format(month)))));
  }

  const tagCounts = new Map();
  for (const item of data.spotlights) {
    for (const hashtag of new Set(item.spotlight.hashtags.map((name) => name.toLowerCase()))) tagCounts.set(hashtag, (tagCounts.get(hashtag) || 0) + 1);
  }
  const topTags = [...tagCounts.entries()].sort((a, b) => b[1] - a[1]).slice(0, 8);
  const tagCard = topTags.length
    ? h('div', { class: 'card' }, h('h3', null, 'Hashtags used most'), h('div', { class: 'bars' }, topTags.map(([name, n]) => h('div', { class: 'bar' }, h('span', { class: 'label selectable', dir: 'auto', text: name }), h('span', { class: 'track' }, h('span', { class: 'fill', vars: { '--v': n / topTags[0][1] } })), h('span', { class: 'n', text: String(n) })))))
    : null;

  return h('div', null, tiles, h('div', { class: 'columns' }, h('div', { class: 'stack-col' }, dated.length ? heatCard : null, monthCard), h('div', { class: 'stack-col' }, tagCard, viewed.length ? null : h('div', { class: 'card' }, h('h3', null, 'Views'), h('p', { text: 'Snapchat doesn\'t show view counts for this profile\'s Spotlights.' })))));
}

function tabBitmoji() {
  const profile = state.report.profile;
  if (!safe(profile.avatar_url)) return empty('user', 'No Bitmoji', 'This profile doesn\'t show a 3D Bitmoji.');
  const current = { key: 'bitmoji:current', section: 'bitmoji', type: 'image', media: profile.avatar_url, thumb: profile.avatar_url, title: 'Current Bitmoji', download: { url: profile.avatar_url, ext: 'webp', category: 'bitmoji', uploaded_at: null } };
  const looks = state.looks;
  const items = looks ? looks.map((look) => ({ key: `bitmoji:${look.md5}`, section: 'bitmoji', type: 'image', media: look.url, thumb: look.url, title: `Version ${look.version}`, look, download: { url: look.url, ext: 'webp', category: 'bitmoji', uploaded_at: null } })) : [current];
  const find = button('Find earlier looks', {
    icon: 'search',
    kind: 'sm',
    onclick: async () => {
      find.disabled = true;
      find.replaceChildren(h('span', { class: 'spinner' }), 'Looking…');
      try {
        state.looks = await invoke('bitmoji_history', { avatarUrl: profile.avatar_url });
      } catch (error) {
        toast(message(error).message, { kind: 'error' });
      }
      renderTab();
    },
  });
  return h(
    'div',
    null,
    toolbar('Bitmoji', looks ? plural(looks.length, 'different look') : 'The 3D Bitmoji this profile shows', looks ? null : find, downloadAll(items)),
    looks ? h('div', { class: 'note' }, icon('info', 'sm'), 'Snapchat keeps earlier versions of a Bitmoji online. These are the distinct looks, newest first.') : null,
    h('div', { class: 'grid' }, items.map((item, index) => tile({ thumb: item.thumb, title: item.title, covers: [item], open: () => openViewer(items, index, { kicker: 'Bitmoji' }) }))),
  );
}

// ---------------------------------------------------------------------------------------------
// The viewer

let viewer = null;

function closeViewer() {
  if (!viewer) return;
  viewer.element.querySelector('video')?.pause();
  viewer.element.remove();
  document.removeEventListener('keydown', viewerKeys, true);
  viewer = null;
}

function viewerKeys(event) {
  if (!viewer || panel || menu) return;
  if (event.key === 'ArrowRight') step(1);
  else if (event.key === 'ArrowLeft') step(-1);
  else if (event.key === ' ' && document.activeElement?.tagName !== 'BUTTON') {
    const video = viewer.element.querySelector('video');
    if (!video) return;
    if (video.paused) video.play();
    else video.pause();
  } else return;
  event.preventDefault();
}

function step(by) {
  const next = viewer.index + by;
  if (next < 0 || next >= viewer.items.length) return;
  viewer.index = next;
  renderViewer();
}

function openViewer(items, index, context = {}) {
  closeViewer();
  closeMenu();
  const stage = h('div', { class: 'stage' });
  const side = h('aside', { class: 'side' });
  const element = h('div', { class: 'viewer', role: 'dialog', 'aria-modal': 'true', 'aria-label': 'Viewer' }, stage, side);
  stage.addEventListener('click', (event) => {
    if (event.target === stage) closeViewer();
  });
  viewer = { items, index, context, element, stage, side };
  $layer.append(element);
  document.addEventListener('keydown', viewerKeys, true);
  renderViewer();
}

function renderViewer() {
  const { items, index, context, stage, side } = viewer;
  const item = items[index];
  const fallback = () =>
    h('div', { class: 'fallback' }, icon('alert', 'xl'), h('p', { text: 'This can\'t be played here. It may have expired, or this computer may be missing a video codec.' }), safe(item.media) ? button('Open in the browser', { icon: 'external', onclick: () => openLink(item.media) }) : null);
  let media;
  if (item.type === 'video' && safe(item.media)) {
    media = h('video', { class: 'media', src: item.media, poster: safe(item.thumb), controls: true, autoplay: true, playsinline: true, preload: 'auto' });
    media.addEventListener('error', () => media.replaceWith(fallback()), { once: true });
    if (context.sequence) media.addEventListener('ended', () => step(1));
  } else if (safe(item.media || item.thumb)) {
    media = h('img', { class: 'media', src: item.media || item.thumb, alt: item.title || '', referrerpolicy: 'no-referrer', draggable: 'false' });
    media.addEventListener('error', () => media.replaceWith(fallback()), { once: true });
  } else {
    media = fallback();
  }
  stage.replaceChildren(
    context.sequence && items.length > 1 && items.length <= 60 ? h('div', { class: 'segments' }, items.map((_, at) => h('i', { class: at <= index ? 'on' : '' }))) : null,
    h('button', { class: 'nav prev', type: 'button', title: 'Previous', 'aria-label': 'Previous', disabled: index === 0, onclick: () => step(-1) }, icon('left', 'lg')),
    media,
    h('button', { class: 'nav next', type: 'button', title: 'Next', 'aria-label': 'Next', disabled: index === items.length - 1, onclick: () => step(1) }, icon('right', 'lg')),
  );

  const video = media.tagName === 'VIDEO' ? media : null;
  const details = (rows) => h('dl', { class: 'details selectable' }, rows.filter(([, value]) => value).map(([label, value]) => [h('dt', { text: label }), h('dd', { dir: 'auto' }, value)]));
  const body = [];
  let page = null;
  if (item.spotlight) {
    const s = item.spotlight;
    const figure = (value, label, iconName) => (value != null ? h('div', { title: fullNumber.format(value) }, h('b', { text: compact(value) }), h('span', null, icon(iconName, 'sm'), label)) : null);
    const figures = [figure(s.views, 'Views', 'eye'), figure(s.comments, 'Comments', 'message'), figure(s.shares, 'Shares', 'share'), figure(s.boosts, 'Boosts', 'bolt'), figure(s.recommends, 'Recommends', 'heart')].filter(Boolean);
    if (figures.length) body.push(h('div', { class: 'kv' }, figures));
    body.push(details([['Posted', moment(item.uploaded_at)], ['Length', length(s.duration_seconds)], ['Size', s.width && s.height ? `${s.width} × ${s.height}` : null], ['Sound', s.sound ? [s.sound.title, s.sound.artist].filter(Boolean).join(' · ') : null]]));
    if (s.hashtags.length) body.push(h('div', { class: 'block' }, h('h4', null, icon('hash', 'sm'), 'Hashtags'), h('div', { class: 'chips selectable' }, s.hashtags.map((name) => h('span', { class: 'chip', dir: 'auto', text: name })))));
    if (s.ai_title || s.ai_summary) body.push(h('div', { class: 'block ai selectable' }, h('h4', null, icon('sparkles', 'sm'), 'Snapchat\'s AI summary'), s.ai_title ? h('b', { dir: 'auto', text: s.ai_title }) : null, s.ai_summary ? h('p', { dir: 'auto', text: s.ai_summary }) : null));
    if (s.keywords.length) body.push(h('div', { class: 'block' }, h('h4', null, 'Topics'), h('div', { class: 'chips selectable' }, s.keywords.map((name) => h('span', { class: 'chip', dir: 'auto', text: name })))));
    page = s.page_url;
  } else if (item.lens) {
    body.push(details([['Made by', item.lens.creator], ['Updated', day(item.lens.updated_at)], ['Kind', item.lens.official ? 'Official Snapchat lens' : 'Community lens']]));
    page = item.lens.unlock_url;
  } else if (item.look) {
    body.push(details([['Version', String(item.look.version)], ['File size', size(item.look.bytes)]]));
  } else {
    body.push(details([['Posted', moment(item.uploaded_at)], ['Kind', item.type === 'image' ? 'Photo' : 'Video'], ['Position', item.position]]));
  }
  if (safe(item.transcript_url)) body.push(captions(item, video));

  side.replaceChildren(
    h(
      'div',
      { class: 'side-head' },
      h('div', { class: 'titles' }, h('div', { class: 'kicker', text: `${context.kicker || 'Item'}${items.length > 1 ? ` · ${index + 1} of ${items.length}` : ''}` }), h('h2', { class: 'selectable', dir: 'auto', text: item.title || '' })),
      h('button', { class: 'btn ghost icon-only', type: 'button', title: 'Close (Esc)', 'aria-label': 'Close', onclick: closeViewer }, icon('x')),
    ),
    h('div', { class: 'side-body' }, body),
    h(
      'div',
      { class: 'side-foot' },
      item.download ? button('Download', { kind: 'primary', icon: 'download', onclick: () => startDownload([item.download]) }) : null,
      safe(page) ? button(item.lens ? 'Open in Snapchat' : 'Open on Snapchat', { icon: 'external', onclick: () => openLink(page) }) : null,
    ),
  );
}

function captions(item, video) {
  const block = h('div', { class: 'block' });
  const load = button('Show captions', {
    icon: 'text',
    kind: 'sm',
    onclick: async () => {
      load.disabled = true;
      load.replaceChildren(h('span', { class: 'spinner' }), 'Loading…');
      try {
        const cues = await invoke('transcript', { url: item.transcript_url });
        if (!cues.length) {
          block.replaceChildren(h('h4', null, icon('text', 'sm'), 'Captions'), h('p', { text: 'The caption file is empty.' }));
          return;
        }
        const stamp = (seconds) => length(Math.floor(seconds));
        const rows = cues.map((cue) => {
          const row = h('button', { class: 'cue', type: 'button', onclick: () => { if (video) { video.currentTime = cue.start; video.play(); } } }, h('time', { text: stamp(cue.start) }), h('span', { class: 'selectable', dir: 'auto', text: cue.text }));
          row.cue = cue;
          return row;
        });
        if (video) {
          video.addEventListener('timeupdate', () => {
            for (const row of rows) row.classList.toggle('now', video.currentTime >= row.cue.start && video.currentTime < row.cue.end);
          });
        }
        block.replaceChildren(h('h4', null, icon('text', 'sm'), 'Captions', h('span', { class: 'spacer' }), h('button', { class: 'btn ghost sm', type: 'button', onclick: () => copy(cues.map((cue) => cue.text).join('\n'), 'Captions copied') }, icon('copy', 'sm'), 'Copy')), h('div', { class: 'cues' }, rows));
      } catch (error) {
        load.disabled = false;
        load.replaceChildren(icon('text'), 'Show captions');
        toast(message(error).message, { kind: 'error' });
      }
    },
  });
  append(block, [h('h4', null, icon('text', 'sm'), 'Captions'), load]);
  return block;
}

// ---------------------------------------------------------------------------------------------
// Downloads

function updateDownloadButton() {
  const element = $topbar.querySelector('.dl-button');
  if (!element) return;
  const job = state.job;
  element.classList.toggle('busy', Boolean(job?.running));
  element.classList.toggle('done', Boolean(job && !job.running && !job.seen));
  element.style.setProperty('--p', job?.total ? job.done / job.total : 0);
}

async function startDownload(files) {
  files = files.filter(Boolean);
  if (!files.length) return;
  if (state.job?.running) {
    toast('A download is already running. This one can start when it finishes.');
    if (!viewer) openDownloads();
    return;
  }
  const username = state.data.user;
  const job = { username, running: true, done: 0, total: files.length, events: [], summary: null, error: null, seen: false };
  state.job = job;
  state.verify = null;
  const quiet = Boolean(viewer) || files.length === 1;
  if (!quiet) openDownloads();
  updateDownloads();
  try {
    job.summary = await invoke('download', { username, items: files });
    job.total = job.summary.total;
    job.done = job.summary.total;
    const s = job.summary;
    const saved = s.downloaded;
    const text = s.cancelled ? `Stopped. ${plural(saved, 'file')} saved.` : s.failed ? `${plural(saved, 'file')} saved, ${s.failed} failed.` : saved ? `${plural(saved, 'file')} saved.` : 'Already saved: nothing new to download.';
    const last = job.events.find((event) => event.outcome !== 'failed' && event.outcome !== 'cancelled');
    const separator = s.directory.includes('\\') ? '\\' : '/';
    const target = s.total === 1 && last ? `${s.directory}${separator}${last.file}` : s.directory;
    if (quiet || panel?.kind !== 'downloads') toast(text, { kind: s.failed ? 'error' : 'success', action: { label: 'Show', run: () => showInFolder(target) } });
  } catch (error) {
    job.error = message(error).message;
    toast(job.error, { kind: 'error' });
  }
  job.running = false;
  job.seen = panel?.kind === 'downloads';
  updateDownloads();
}

function updateDownloads() {
  updateDownloadButton();
  if (panel?.kind === 'downloads') panel.element.querySelector('.drawer-body').replaceChildren(...downloadsBody());
}

function openDownloads() {
  if (state.job) state.job.seen = !state.job.running;
  const drawer = h('div', { class: 'drawer', role: 'dialog', 'aria-label': 'Downloads' }, panelHead('Downloads', 'download'), h('div', { class: 'drawer-body' }, downloadsBody()));
  openPanel('downloads', drawer);
  updateDownloadButton();
}

function downloadsBody() {
  const job = state.job;
  const username = job?.username || state.data?.user;
  const parts = [];
  if (job) {
    const s = job.summary;
    const head = job.running ? `Saving ${plural(job.total, 'file')} from @${job.username}` : job.error ? 'The download stopped' : s?.cancelled ? `Stopped: @${job.username}` : `Done: @${job.username}`;
    const pill = (n, label, kind = '') => (n ? h('span', { class: `pill ${kind}`.trim(), text: `${n} ${label}` }) : null);
    parts.push(
      h(
        'div',
        { class: 'job' },
        h('div', { class: 'job-head' }, job.running ? h('span', { class: 'spinner' }) : icon(job.error || s?.failed ? 'alert' : 'check'), h('b', { text: head }), job.running ? button('Stop', { kind: 'sm', onclick: () => invoke('cancel_download') }) : null),
        h('div', { class: 'progress' }, h('i', { vars: { '--p': job.total ? job.done / job.total : 0 } })),
        h('div', { class: 'counts' }, h('span', { class: 'pill', text: `${job.done} of ${job.total}` }), s ? [pill(s.downloaded, 'saved', 'ok'), pill(s.skipped + s.already_present, 'already saved'), pill(s.saved_beside, 'kept beside a changed file', 'warn'), pill(s.failed, 'failed', 'bad'), s.bytes ? h('span', { class: 'pill', text: size(s.bytes) }) : null] : null),
        job.error ? h('div', { class: 'verify-result bad selectable', text: job.error }) : null,
        s?.altered_on_disk.length ? h('div', { class: 'verify-result bad selectable' }, h('b', { text: 'Changed since they were downloaded, so left untouched:' }), s.altered_on_disk.map((name) => h('code', { text: name }))) : null,
        s?.errors.length ? h('div', { class: 'verify-result bad selectable' }, s.errors.slice(0, 5).map((line) => h('code', { text: line }))) : null,
        job.events.length
          ? h('div', { class: 'events' }, job.events.slice(0, 60).map((event) => h('div', { class: 'event' }, icon(event.outcome === 'failed' ? 'alert' : event.outcome === 'cancelled' ? 'x' : 'check', `sm ${event.outcome === 'failed' ? 'bad' : event.outcome === 'cancelled' ? '' : 'ok'}`), h('code', { text: event.file, title: event.file }), { downloaded: size(event.bytes), skipped: 'already saved', already_present: 'already there', saved_beside: 'saved beside', failed: 'failed', cancelled: 'stopped' }[event.outcome])))
          : null,
      ),
    );
  }
  if (username) {
    const path = h('div', { class: 'path selectable', text: '…' });
    invoke('account_folder', { username }).then((folder) => { path.textContent = folder; }).catch(() => {});
    const result = h('div');
    if (state.verify) result.replaceChildren(verifyResult(state.verify));
    const check = button('Check files', {
      icon: 'shield',
      kind: 'sm',
      title: 'Compare every saved file with the hash recorded when it was downloaded',
      onclick: async () => {
        check.disabled = true;
        try {
          state.verify = await invoke('verify_downloads', { username });
          result.replaceChildren(verifyResult(state.verify));
        } catch (error) {
          toast(message(error).message, { kind: 'error' });
        }
        check.disabled = false;
      },
    });
    parts.push(h('div', { class: 'job' }, h('div', { class: 'job-head' }, icon('folder'), h('b', { text: `Folder for @${username}` })), path, h('div', { class: 'counts' }, button('Open folder', { icon: 'external', kind: 'sm', onclick: () => showInFolder(path.textContent) }), check), result));
  }
  if (!parts.length) parts.push(empty('download', 'Nothing here yet', 'Look a profile up and download its story, highlights, Spotlights or lenses. Progress and results show here.'));
  return parts;
}

function verifyResult(report) {
  if (!report.has_manifest) return h('div', { class: 'verify-result neutral', text: 'Nothing has been downloaded to this folder yet.' });
  const list = (label, names) => (names.length ? [h('b', { text: label }), names.slice(0, 12).map((name) => h('code', { text: name })), names.length > 12 ? `and ${names.length - 12} more` : null] : null);
  const intact = !report.altered.length && !report.missing.length;
  return h(
    'div',
    { class: `verify-result selectable ${intact ? 'good' : 'bad'}` },
    h('b', { text: intact ? `All ${plural(report.ok, 'file')} match what was downloaded.` : `${report.altered.length + report.missing.length} of ${report.ok + report.altered.length + report.missing.length} files don't match the record.` }),
    list('Changed:', report.altered),
    list('Missing:', report.missing),
    report.untracked.length ? `${plural(report.untracked.length, 'media file')} in the folder ${report.untracked.length === 1 ? 'isn\'t' : 'aren\'t'} in the record.` : null,
  );
}

async function exportReport(format) {
  try {
    const path = await invoke('export_report', { format });
    if (path) toast(`Exported as ${format.toUpperCase()}`, { kind: 'success', action: { label: 'Show', run: () => showInFolder(path) } });
  } catch (error) {
    toast(message(error).message, { kind: 'error' });
  }
}

// ---------------------------------------------------------------------------------------------
// Start

async function start() {
  renderTopbar();
  await loadSettings();
  render();
  await tauri.event.listen('download-progress', (event) => {
    const job = state.job;
    if (!job?.running) return;
    job.done = event.payload.done;
    job.total = event.payload.total;
    job.events.unshift(event.payload);
    updateDownloads();
  });
  const first = await invoke('startup_username').catch(() => null);
  if (first) lookup(first);
  else focusSearch();
}

start();
