// Printer Selector — injected by Tauri on every ticket page load
(function () {
  if (window.__TAURI_PRINTER_SELECTOR__) return;
  window.__TAURI_PRINTER_SELECTOR__ = true;

  // ── Styles ────────────────────────────────────────────────────────────────
  const style = document.createElement('style');
  style.textContent = `
    #tpr-wrap {
      position: relative;
      display: flex;
      align-items: center;
    }

    #tpr-btn {
      display: flex;
      align-items: center;
      gap: 0.375rem;
      padding: 0.375rem 0.625rem;
      background: none;
      border: 1px solid var(--border, #e4e4e7);
      border-radius: 0.5rem;
      color: var(--text-secondary, #71717a);
      font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
      font-size: 0.8125rem;
      font-weight: 500;
      cursor: pointer;
      transition: color 0.15s ease, background 0.15s ease, border-color 0.15s ease;
      white-space: nowrap;
    }
    #tpr-btn:hover {
      color: var(--accent, #0ea5e9);
      background: var(--accent-soft, #e0f2fe);
      border-color: var(--accent, #0ea5e9);
    }
    #tpr-btn svg { flex-shrink: 0; }

    /* Dropdown panel */
    #tpr-panel {
      display: none;
      position: fixed;
      width: 260px;
      background: var(--bg-secondary, #fff);
      border: 1px solid var(--border, #e4e4e7);
      border-radius: 1rem;
      box-shadow: var(--shadow-lg, 0 12px 40px rgba(0,0,0,0.08));
      z-index: 9001;
      overflow: hidden;
      animation: tpr-in 0.14s ease;
    }
    @keyframes tpr-in {
      from { opacity: 0; transform: translateY(-6px); }
      to   { opacity: 1; transform: translateY(0);    }
    }
    #tpr-panel.open { display: block; }

    #tpr-panel-header {
      padding: 0.75rem 1rem;
      border-bottom: 1px solid var(--border, #e4e4e7);
      display: flex;
      justify-content: space-between;
      align-items: center;
    }
    #tpr-panel-header span {
      font-family: 'Inter', sans-serif;
      font-size: 0.8125rem;
      font-weight: 600;
      color: var(--text-primary, #18181b);
    }
    #tpr-close {
      background: none;
      border: none;
      cursor: pointer;
      color: var(--text-muted, #a1a1aa);
      font-size: 1rem;
      line-height: 1;
      padding: 0;
      transition: color 0.15s;
    }
    #tpr-close:hover { color: var(--text-primary, #18181b); }

    #tpr-current {
      padding: 0.5rem 1rem;
      font-family: 'Inter', sans-serif;
      font-size: 0.75rem;
      color: var(--text-muted, #a1a1aa);
      background: var(--bg-accent, #f5f5f5);
      border-bottom: 1px solid var(--border, #e4e4e7);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    #tpr-current strong {
      color: var(--accent, #0ea5e9);
      font-weight: 600;
    }

    #tpr-list { max-height: 196px; overflow-y: auto; }

    .tpr-item {
      display: flex;
      align-items: center;
      gap: 0.625rem;
      padding: 0.625rem 1rem;
      font-family: 'Inter', sans-serif;
      font-size: 0.8125rem;
      color: var(--text-secondary, #71717a);
      cursor: pointer;
      border-bottom: 1px solid var(--bg-accent, #f5f5f5);
      transition: background 0.12s ease, color 0.12s ease;
    }
    .tpr-item:last-child { border-bottom: none; }
    .tpr-item:hover {
      background: var(--accent-soft, #e0f2fe);
      color: var(--accent, #0ea5e9);
    }
    .tpr-item.active {
      color: var(--accent, #0ea5e9);
      font-weight: 600;
      background: var(--accent-soft, #e0f2fe);
    }
    .tpr-check {
      font-size: 0.75rem;
      min-width: 12px;
      text-align: center;
      color: var(--accent, #0ea5e9);
    }

    #tpr-status {
      padding: 0.375rem 1rem;
      font-family: 'Inter', sans-serif;
      font-size: 0.75rem;
      color: var(--text-muted, #a1a1aa);
      text-align: center;
      min-height: 1.75rem;
      border-top: 1px solid var(--border, #e4e4e7);
    }

    #tpr-reload {
      display: block;
      width: 100%;
      padding: 0.5rem 1rem;
      background: none;
      border: none;
      border-top: 1px solid var(--border, #e4e4e7);
      font-family: 'Inter', sans-serif;
      font-size: 0.75rem;
      color: var(--accent, #0ea5e9);
      cursor: pointer;
      transition: background 0.12s ease;
      text-align: center;
    }
    #tpr-reload:hover { background: var(--accent-soft, #e0f2fe); }

    /* Paper settings section */
    #tpr-paper {
      border-top: 1px solid var(--border, #e4e4e7);
      padding: 0.75rem 1rem 0.5rem;
    }
    #tpr-paper-title {
      font-family: 'Inter', sans-serif;
      font-size: 0.6875rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--text-muted, #a1a1aa);
      margin-bottom: 0.5rem;
    }
    .tpr-paper-options {
      display: flex;
      gap: 0.5rem;
      margin-bottom: 0.625rem;
    }
    .tpr-paper-opt {
      flex: 1;
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 2px;
      padding: 0.375rem 0.25rem;
      border: 1.5px solid var(--border, #e4e4e7);
      border-radius: 0.5rem;
      cursor: pointer;
      transition: border-color 0.15s, background 0.15s;
      font-family: 'Inter', sans-serif;
    }
    .tpr-paper-opt.active {
      border-color: var(--accent, #0ea5e9);
      background: var(--accent-soft, #e0f2fe);
    }
    .tpr-paper-opt-label  { font-size: 0.8125rem; font-weight: 600; color: var(--text-primary, #18181b); }
    .tpr-paper-opt-sub    { font-size: 0.6875rem; color: var(--text-muted, #a1a1aa); }
    .tpr-feed-row {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      font-family: 'Inter', sans-serif;
      font-size: 0.8125rem;
      color: var(--text-secondary, #71717a);
      margin-bottom: 0.375rem;
    }
    .tpr-feed-row label { flex: 1; }
    .tpr-feed-row input[type=range] { flex: 1.5; accent-color: var(--accent, #0ea5e9); }
    .tpr-feed-val { min-width: 1.25rem; text-align: center; font-weight: 600; color: var(--text-primary, #18181b); }
    #tpr-paper-save {
      display: block;
      width: 100%;
      padding: 0.375rem 1rem;
      background: var(--accent, #0ea5e9);
      color: #fff;
      border: none;
      border-radius: 0.375rem;
      font-family: 'Inter', sans-serif;
      font-size: 0.75rem;
      font-weight: 600;
      cursor: pointer;
      transition: background 0.12s;
      margin-top: 0.5rem;
    }
    #tpr-paper-save:hover { background: #0284c7; }
    #tpr-paper-status {
      margin-top: 0.375rem;
      font-family: 'Inter', sans-serif;
      font-size: 0.75rem;
      text-align: center;
      min-height: 1.25rem;
      color: var(--text-muted, #a1a1aa);
    }

    /* Server URL section */
    #tpr-server {
      border-top: 1px solid var(--border, #e4e4e7);
      padding: 0.75rem 1rem 0.75rem;
    }
    #tpr-server-title {
      font-family: 'Inter', sans-serif;
      font-size: 0.6875rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--text-muted, #a1a1aa);
      margin-bottom: 0.5rem;
    }
    #tpr-server-url {
      width: 100%;
      box-sizing: border-box;
      padding: 0.375rem 0.5rem;
      border: 1.5px solid var(--border, #e4e4e7);
      border-radius: 0.375rem;
      font-family: 'Inter', monospace, sans-serif;
      font-size: 0.75rem;
      color: var(--text-primary, #18181b);
      background: var(--bg-secondary, #fff);
      outline: none;
      transition: border-color 0.15s;
      margin-bottom: 0.5rem;
    }
    #tpr-server-url:focus { border-color: var(--accent, #0ea5e9); }
    #tpr-server-save {
      display: block;
      width: 100%;
      padding: 0.375rem 1rem;
      background: var(--accent, #0ea5e9);
      color: #fff;
      border: none;
      border-radius: 0.375rem;
      font-family: 'Inter', sans-serif;
      font-size: 0.75rem;
      font-weight: 600;
      cursor: pointer;
      transition: background 0.12s;
    }
    #tpr-server-save:hover { background: #0284c7; }
    #tpr-server-status {
      margin-top: 0.375rem;
      font-family: 'Inter', sans-serif;
      font-size: 0.75rem;
      text-align: center;
      min-height: 1.25rem;
      color: var(--text-muted, #a1a1aa);
    }
  `;
  document.head.appendChild(style);

  // ── Printer icon (matches the minimal line style of the page) ─────────────
  const printerIcon = `<svg width="14" height="14" viewBox="0 0 24 24" fill="none"
    stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <polyline points="6 9 6 2 18 2 18 9"/>
    <path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"/>
    <rect x="6" y="14" width="12" height="8"/>
  </svg>`;

  // ── Build DOM ─────────────────────────────────────────────────────────────
  const wrap = document.createElement('div');
  wrap.id = 'tpr-wrap';

  const btn = document.createElement('button');
  btn.id = 'tpr-btn';
  btn.title = 'Pilih Printer';
  btn.innerHTML = printerIcon + '<span>Printer</span>';
  wrap.appendChild(btn);

  const panel = document.createElement('div');
  panel.id = 'tpr-panel';
  panel.innerHTML = `
    <div id="tpr-panel-header">
      <span>Printer & Kertas</span>
      <button id="tpr-close" title="Tutup">✕</button>
    </div>
    <div id="tpr-current">Aktif: <strong id="tpr-active">-</strong></div>
    <div id="tpr-list">
      <div style="padding:1rem;text-align:center;font-size:0.8rem;color:var(--text-muted,#a1a1aa);">Memuat...</div>
    </div>
    <div id="tpr-status"></div>
    <button id="tpr-reload">↻ Muat Ulang Daftar</button>
    <div id="tpr-paper">
      <div id="tpr-paper-title">Ukuran Kertas</div>
      <div class="tpr-paper-options">
        <div class="tpr-paper-opt active" id="tpr-opt-80" data-size="80mm">
          <span class="tpr-paper-opt-label">80 mm</span>
          <span class="tpr-paper-opt-sub">Standar</span>
        </div>
        <div class="tpr-paper-opt" id="tpr-opt-58" data-size="58mm">
          <span class="tpr-paper-opt-label">58 mm</span>
          <span class="tpr-paper-opt-sub">Kiosk kecil</span>
        </div>
      </div>
      <div class="tpr-feed-row">
        <label>Spasi sebelum potong</label>
        <input type="range" id="tpr-feed-range" min="1" max="5" step="1" value="1"
          oninput="document.getElementById('tpr-feed-val').textContent=this.value">
        <span class="tpr-feed-val" id="tpr-feed-val">1</span>
      </div>
      <button id="tpr-paper-save">Simpan Pengaturan Kertas</button>
      <div id="tpr-paper-status"></div>
    </div>
    <div id="tpr-server">
      <div id="tpr-server-title">Alamat Server</div>
      <input type="text" id="tpr-server-url" placeholder="http://localhost:8080"
        autocomplete="off" autocorrect="off" spellcheck="false">
      <button id="tpr-server-save">Simpan & Pindah ke Server</button>
      <div id="tpr-server-status"></div>
    </div>
  `;
  document.body.appendChild(panel);

  // Inject button into .ticket-header (natural flex item)
  const header = document.querySelector('.ticket-header');
  if (header) {
    header.appendChild(wrap);
  } else {
    // Fallback: top-right fixed if header not found
    wrap.style.cssText = 'position:fixed;top:1rem;right:1rem;z-index:9000;';
    document.body.appendChild(wrap);
  }

  const invoke = window.__TAURI__.core.invoke;

  // ── Position panel below button ───────────────────────────────────────────
  function positionPanel() {
    const rect = btn.getBoundingClientRect();
    panel.style.top  = (rect.bottom + 8) + 'px';
    panel.style.right = (window.innerWidth - rect.right) + 'px';
  }

  // ── Load printers ─────────────────────────────────────────────────────────
  async function loadPrinters() {
    const list   = document.getElementById('tpr-list');
    const status = document.getElementById('tpr-status');
    list.innerHTML = '<div style="padding:1rem;text-align:center;font-size:0.8rem;color:var(--text-muted,#a1a1aa);">Memuat...</div>';
    status.textContent = '';
    status.style.color = '';

    try {
      const [printers, current] = await Promise.all([
        invoke('list_printers'),
        invoke('get_printer_config'),
      ]);

      document.getElementById('tpr-active').textContent = current || '(belum dipilih)';
      // Update button label to show current printer (truncated)
      const label = btn.querySelector('span');
      if (label) {
        label.textContent = current ? truncate(current, 16) : 'Printer';
      }

      if (!printers || printers.length === 0) {
        list.innerHTML = '<div style="padding:1rem;text-align:center;font-size:0.8rem;color:var(--text-muted,#a1a1aa);">Tidak ada printer ditemukan</div>';
        return;
      }

      list.innerHTML = printers.map(p => `
        <div class="tpr-item ${p === current ? 'active' : ''}" data-name="${esc(p)}">
          <span class="tpr-check">${p === current ? '✓' : ''}</span>
          <span>${esc(p)}</span>
        </div>
      `).join('');

      list.querySelectorAll('.tpr-item').forEach(item => {
        item.addEventListener('click', () => selectPrinter(item.dataset.name));
      });
    } catch (e) {
      list.innerHTML = '<div style="padding:1rem;text-align:center;font-size:0.8rem;color:#ef4444;">Gagal memuat daftar printer</div>';
    }
  }

  // ── Select printer ────────────────────────────────────────────────────────
  async function selectPrinter(name) {
    const status = document.getElementById('tpr-status');
    status.textContent = 'Menyimpan...';
    status.style.color = 'var(--text-muted, #a1a1aa)';

    try {
      await invoke('save_printer', { printerName: name });

      document.getElementById('tpr-active').textContent = name;
      const label = btn.querySelector('span');
      if (label) label.textContent = truncate(name, 16);

      document.querySelectorAll('.tpr-item').forEach(item => {
        const active = item.dataset.name === name;
        item.classList.toggle('active', active);
        item.querySelector('.tpr-check').textContent = active ? '✓' : '';
      });

      status.textContent = '✓ Tersimpan. Print agent dimulai ulang.';
      status.style.color = 'var(--success, #22c55e)';
      setTimeout(() => { status.textContent = ''; status.style.color = ''; }, 3500);
    } catch (e) {
      status.textContent = '✗ ' + (e || 'Gagal menyimpan');
      status.style.color = '#ef4444';
    }
  }

  // ── Utils ─────────────────────────────────────────────────────────────────
  function esc(s) {
    return String(s)
      .replace(/&/g,'&amp;').replace(/</g,'&lt;')
      .replace(/>/g,'&gt;').replace(/"/g,'&quot;');
  }
  function truncate(s, n) {
    return s.length > n ? s.slice(0, n) + '…' : s;
  }

  // ── Events ────────────────────────────────────────────────────────────────
  btn.addEventListener('click', e => {
    e.stopPropagation();
    if (panel.classList.contains('open')) {
      panel.classList.remove('open');
    } else {
      positionPanel();
      panel.classList.add('open');
      loadPrinters();
    }
  });

  document.getElementById('tpr-close').addEventListener('click', () => {
    panel.classList.remove('open');
  });

  document.getElementById('tpr-reload').addEventListener('click', loadPrinters);

  document.addEventListener('click', e => {
    if (panel.classList.contains('open')
        && !panel.contains(e.target)
        && !wrap.contains(e.target)) {
      panel.classList.remove('open');
    }
  });

  window.addEventListener('resize', () => {
    if (panel.classList.contains('open')) positionPanel();
  });

  // ── Paper size option toggle ──────────────────────────────────────────────
  let selectedPaperSize = '80mm';

  function setPaperSize(size) {
    selectedPaperSize = size;
    document.querySelectorAll('.tpr-paper-opt').forEach(el => {
      el.classList.toggle('active', el.dataset.size === size);
    });
  }

  document.querySelectorAll('.tpr-paper-opt').forEach(el => {
    el.addEventListener('click', () => setPaperSize(el.dataset.size));
  });

  // ── Save paper config ─────────────────────────────────────────────────────
  async function savePaperConfig() {
    const btn    = document.getElementById('tpr-paper-save');
    const status = document.getElementById('tpr-paper-status');
    const feedLines = parseInt(document.getElementById('tpr-feed-range').value) || 1;
    btn.disabled = true;
    status.textContent = 'Menyimpan...';
    status.style.color = 'var(--text-muted, #a1a1aa)';
    try {
      await invoke('save_paper_config', { paperSize: selectedPaperSize, feedLines });
      status.textContent = '✓ Tersimpan. Print agent dimulai ulang.';
      status.style.color = 'var(--success, #22c55e)';
      setTimeout(() => { status.textContent = ''; }, 3500);
    } catch (e) {
      status.textContent = '✗ ' + (e || 'Gagal menyimpan');
      status.style.color = '#ef4444';
    }
    btn.disabled = false;
  }

  document.getElementById('tpr-paper-save').addEventListener('click', savePaperConfig);

  // ── Save server URL ───────────────────────────────────────────────────────
  async function saveServerUrl() {
    const saveBtn = document.getElementById('tpr-server-save');
    const status  = document.getElementById('tpr-server-status');
    const urlVal  = (document.getElementById('tpr-server-url').value || '').trim();

    if (!urlVal) {
      status.textContent = '✗ URL tidak boleh kosong';
      status.style.color = '#ef4444';
      return;
    }
    if (!/^https?:\/\/.+/.test(urlVal)) {
      status.textContent = '✗ Harus diawali http:// atau https://';
      status.style.color = '#ef4444';
      return;
    }

    saveBtn.disabled = true;
    status.textContent = 'Menyimpan & memuat ulang...';
    status.style.color = 'var(--text-muted, #a1a1aa)';
    try {
      await invoke('save_server_url', { serverUrl: urlVal });
      // Page will navigate — update badge immediately
      const el = document.getElementById('tsi-url');
      if (el) el.textContent = urlVal;
      status.textContent = '✓ Tersimpan. Memuat halaman baru...';
      status.style.color = 'var(--success, #22c55e)';
    } catch (e) {
      status.textContent = '✗ ' + (e || 'Gagal menyimpan');
      status.style.color = '#ef4444';
      saveBtn.disabled = false;
    }
  }

  document.getElementById('tpr-server-save').addEventListener('click', saveServerUrl);

  // Allow pressing Enter in the URL field
  document.getElementById('tpr-server-url').addEventListener('keydown', e => {
    if (e.key === 'Enter') saveServerUrl();
  });

  // ── Show current printer on initial load ──────────────────────────────────
  invoke('get_printer_config').then(current => {
    if (current) {
      const label = btn.querySelector('span');
      if (label) label.textContent = truncate(current, 16);
    }
  }).catch(() => {});

  // Load current paper config
  invoke('get_paper_config').then(([paperSize, feedLines]) => {
    setPaperSize(paperSize || '80mm');
    const feedEl = document.getElementById('tpr-feed-range');
    const valEl  = document.getElementById('tpr-feed-val');
    if (feedEl) feedEl.value = feedLines || 1;
    if (valEl)  valEl.textContent = feedLines || 1;
  }).catch(() => {});

  // Load current server URL into the input field
  invoke('get_server_url').then(url => {
    const el = document.getElementById('tpr-server-url');
    if (el && url) el.value = url;
  }).catch(() => {});

  // ── Server info badge ─────────────────────────────────────────────────────
  const siStyle = document.createElement('style');
  siStyle.textContent = `
    #tsi-badge {
      display: inline-flex;
      align-items: center;
      gap: 0.3rem;
      font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
      font-size: 0.7rem;
      color: #a1a1aa;
      background: #f5f5f5;
      border: 1px solid #e4e4e7;
      border-radius: 0.375rem;
      padding: 0.2rem 0.5rem;
      white-space: nowrap;
      user-select: none;
      margin-top: 0.4rem;
    }
    #tsi-badge svg { flex-shrink: 0; opacity: 0.7; }
  `;
  document.head.appendChild(siStyle);

  const siBadge = document.createElement('div');
  siBadge.id = 'tsi-badge';
  siBadge.title = 'Server yang sedang digunakan';
  siBadge.innerHTML = `<svg width="10" height="10" viewBox="0 0 24 24" fill="none"
    stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
    <rect x="2" y="2" width="20" height="8" rx="2" ry="2"/>
    <rect x="2" y="14" width="20" height="8" rx="2" ry="2"/>
    <line x1="6" y1="6" x2="6.01" y2="6"/>
    <line x1="6" y1="18" x2="6.01" y2="18"/>
  </svg><span id="tsi-url">memuat...</span>`;

  const siFooter = document.querySelector('.ticket-footer');
  if (siFooter) {
    siFooter.appendChild(siBadge);
  } else {
    siBadge.style.cssText = 'position:fixed;bottom:0.5rem;left:50%;transform:translateX(-50%);z-index:8999;';
    document.body.appendChild(siBadge);
  }

  invoke('get_server_url').then(url => {
    const el = document.getElementById('tsi-url');
    if (el) el.textContent = url;
  }).catch(() => {
    const el = document.getElementById('tsi-url');
    if (el) el.textContent = 'tidak diketahui';
  });
})();
