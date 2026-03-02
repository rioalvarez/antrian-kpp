// Super Counter – Admin-only multi-loket control page

let selectedTypeCode   = null;
let selectedTypeName   = null;
let selectedTypePrefix = null;
let selectedCounterId  = null;
let hasCurrentQueue    = false;
let eventSource        = null;

// ── Init ──────────────────────────────────────────────────────

document.addEventListener('DOMContentLoaded', function () {
    loadStatsByType();
    loadAllCounters();

    setInterval(loadStatsByType, 3000);
    setInterval(function () {
        loadAllCounters();
        // Jika di state-3, refresh data counter yang dipilih
        if (selectedCounterId) loadCounterData();
    }, 3000);
});

// ── Navigasi State ────────────────────────────────────────────

function showState(n) {
    document.getElementById('state-1').style.display = n === 1 ? 'flex'  : 'none';
    document.getElementById('state-2').style.display = n === 2 ? 'block' : 'none';
    document.getElementById('state-3').style.display = n === 3 ? 'flex'  : 'none';
}

function goToState1() {
    selectedTypeCode   = null;
    selectedTypeName   = null;
    selectedTypePrefix = null;
    goToState2(); // clear counter juga
    document.querySelectorAll('.sc-type-card').forEach(c => c.classList.remove('selected'));
    showState(1);
}

function goToState2() {
    selectedCounterId = null;
    hasCurrentQueue   = false;
    document.querySelectorAll('.sc-loket-card').forEach(c => c.classList.remove('selected'));
    disconnectSSE();
    // Refresh loket sebelum tampil
    loadAllCounters();
    showState(2);
}

// ── Pilih Jenis Antrian (state 1 → 2) ────────────────────────

function selectQueueType(code, name, prefix) {
    selectedTypeCode   = code;
    selectedTypeName   = name;
    selectedTypePrefix = prefix;

    // Tandai kartu yang dipilih
    document.querySelectorAll('.sc-type-card').forEach(c => {
        c.classList.toggle('selected', c.dataset.code === code);
    });

    // Isi info di state-2
    document.getElementById('s2-prefix').textContent    = prefix;
    document.getElementById('s2-type-name').textContent = name;
    updateWaitingPill();

    showState(2);
    loadAllCounters();
}

// ── Pilih Loket (state 2 → 3) ────────────────────────────────

function selectCounter(id, name, number) {
    selectedCounterId = id;

    // Tandai loket yang dipilih
    document.querySelectorAll('.sc-loket-card').forEach(c => {
        c.classList.toggle('selected', parseInt(c.dataset.id) === id);
    });

    // Isi header state-3
    document.getElementById('s3-prefix').textContent      = selectedTypePrefix;
    document.getElementById('s3-type-name').textContent   = selectedTypeName;
    document.getElementById('s3-counter-name').textContent = name + ' (Loket ' + number + ')';
    document.getElementById('qi-prefix').textContent      = selectedTypePrefix;

    // Tampilkan state-3
    showState(3);

    // Tombol "panggil berikutnya" langsung aktif (jenis sudah dipilih di step 1)
    document.getElementById('btn-next').disabled = false;

    loadCounterData();
    connectSSE(id);
}

// ── Muat Data ─────────────────────────────────────────────────

async function loadStatsByType() {
    try {
        const res = await fetch('/api/stats/by-type');
        if (!res.ok) return;
        const counts = await res.json();

        // Badge di sidebar kiri
        QUEUE_TYPES.forEach(qt => {
            const badge = document.getElementById('badge-' + qt.code);
            if (badge) badge.textContent = counts[qt.code] !== undefined ? counts[qt.code] : 0;
        });

        // Pill di state-2 header
        updateWaitingPill(counts);

        // Counter antrian menunggu di state-3
        if (selectedTypeCode) {
            const el = document.getElementById('qi-count');
            if (el) el.textContent = counts[selectedTypeCode] !== undefined ? counts[selectedTypeCode] : 0;
        }
    } catch (err) {
        console.error('loadStatsByType error:', err);
    }
}

function updateWaitingPill(counts) {
    if (!selectedTypeCode) return;
    const pill = document.getElementById('s2-waiting');
    if (!pill) return;
    let n = 0;
    if (counts && counts[selectedTypeCode] !== undefined) {
        n = counts[selectedTypeCode];
    } else {
        // Baca dari badge yang sudah ada
        const badge = document.getElementById('badge-' + selectedTypeCode);
        if (badge) n = parseInt(badge.textContent) || 0;
    }
    pill.textContent = n + ' menunggu';
    pill.className = 'sc-waiting-pill' + (n > 0 ? ' has-queue' : '');
}

async function loadAllCounters() {
    try {
        const res = await fetch('/api/counters');
        if (!res.ok) return;
        const counters = await res.json();

        counters.forEach(counter => {
            const qEl  = document.getElementById('loket-q-'   + counter.id);
            const dotEl = document.getElementById('loket-dot-' + counter.id);
            if (!qEl) return;

            const hasQueue = counter.current_queue && counter.current_queue.queue_number;
            qEl.textContent  = hasQueue ? counter.current_queue.queue_number : '---';
            if (dotEl) dotEl.className = 'sc-lc-dot' + (hasQueue ? ' active' : '');
        });
    } catch (err) {
        console.error('loadAllCounters error:', err);
    }
}

async function loadCounterData() {
    if (!selectedCounterId) return;
    try {
        const res = await fetch('/api/counter/' + selectedCounterId);
        if (!res.ok) return;
        const counter = await res.json();
        updateControlUI(counter);
    } catch (err) {
        console.error('loadCounterData error:', err);
    }
}

function updateControlUI(counter) {
    const queueEl  = document.getElementById('current-queue');
    const statusEl = document.getElementById('queue-status');
    if (!queueEl) return;

    const hasQueue = counter.current_queue && counter.current_queue.queue_number;
    hasCurrentQueue = !!hasQueue;

    queueEl.textContent = hasQueue ? counter.current_queue.queue_number : '---';
    if (hasQueue) {
        statusEl.textContent = 'Sedang Dilayani';
        statusEl.classList.add('active');
    } else {
        statusEl.textContent = 'Tidak Ada Antrian';
        statusEl.classList.remove('active');
    }

    document.getElementById('btn-recall').disabled   = !hasQueue;
    document.getElementById('btn-complete').disabled = !hasQueue;
    document.getElementById('btn-cancel').disabled   = !hasQueue;
}

// ── SSE ───────────────────────────────────────────────────────

function connectSSE(counterId) {
    disconnectSSE();
    try {
        eventSource = new EventSource('/api/sse/counter/' + counterId);

        eventSource.onopen = () => updateConnStatus(true);
        eventSource.addEventListener('connected', () => updateConnStatus(true));

        eventSource.addEventListener('message', function (e) {
            try {
                const event = JSON.parse(e.data);
                if (['queue_updated', 'queue_added', 'queue_reset'].includes(event.type)) {
                    loadCounterData();
                    loadStatsByType();
                    loadAllCounters();
                }
            } catch {}
        });

        eventSource.onerror = function () {
            updateConnStatus(false);
            eventSource.close();
            setTimeout(function () {
                if (selectedCounterId) connectSSE(selectedCounterId);
            }, 3000);
        };
    } catch {
        updateConnStatus(false);
    }
}

function disconnectSSE() {
    if (eventSource) {
        eventSource.close();
        eventSource = null;
    }
    updateConnStatus(false);
}

function updateConnStatus(connected) {
    const el = document.getElementById('conn-status');
    if (!el) return;
    el.textContent = connected ? 'Terhubung' : (selectedCounterId ? 'Polling Mode' : '—');
    el.className   = 'sc-conn' + (connected ? ' connected' : (selectedCounterId ? ' disconnected' : ''));
}

// ── Aksi Counter ─────────────────────────────────────────────

async function callNext() {
    if (!selectedCounterId || !selectedTypeCode) return;

    const btn = document.getElementById('btn-next');
    btn.disabled = true;

    try {
        const res = await fetch(
            '/api/counter/' + selectedCounterId + '/call-next?type=' + selectedTypeCode,
            { method: 'POST' }
        );
        if (res.status === 404) {
            alert('Tidak ada antrian jenis ' + selectedTypePrefix + ' (' + selectedTypeName + ') yang sedang menunggu.');
            return;
        }
        if (!res.ok) throw new Error('call-next failed');

        const counter = await res.json();
        updateControlUI(counter);
        flashQueueNumber();
    } catch (err) {
        console.error(err);
        alert('Gagal memanggil antrian. Silakan coba lagi.');
    } finally {
        btn.disabled = false;
        loadAllCounters();
        loadStatsByType();
    }
}

async function recall() {
    if (!hasCurrentQueue || !selectedCounterId) return;

    const btn = document.getElementById('btn-recall');
    btn.disabled = true;

    try {
        const res = await fetch('/api/counter/' + selectedCounterId + '/recall', { method: 'POST' });
        if (!res.ok) throw new Error('recall failed');
        flashQueueNumber();
    } catch (err) {
        console.error(err);
        alert('Gagal memanggil ulang. Silakan coba lagi.');
    } finally {
        btn.disabled = false;
    }
}

async function complete() {
    if (!hasCurrentQueue || !selectedCounterId) return;

    const btn = document.getElementById('btn-complete');
    btn.disabled = true;

    try {
        const res = await fetch('/api/counter/' + selectedCounterId + '/complete', { method: 'POST' });
        if (!res.ok) throw new Error('complete failed');
        const counter = await res.json();
        updateControlUI(counter);
    } catch (err) {
        console.error(err);
        alert('Gagal menyelesaikan antrian. Silakan coba lagi.');
    } finally {
        btn.disabled = false;
        loadAllCounters();
        loadStatsByType();
    }
}

async function cancel() {
    if (!hasCurrentQueue || !selectedCounterId) return;
    if (!confirm('Yakin ingin melewati antrian ini?')) return;

    const btn = document.getElementById('btn-cancel');
    btn.disabled = true;

    try {
        const res = await fetch('/api/counter/' + selectedCounterId + '/cancel', { method: 'POST' });
        if (!res.ok) throw new Error('cancel failed');
        const counter = await res.json();
        updateControlUI(counter);
    } catch (err) {
        console.error(err);
        alert('Gagal melewati antrian. Silakan coba lagi.');
    } finally {
        btn.disabled = false;
        loadAllCounters();
        loadStatsByType();
    }
}

function flashQueueNumber() {
    const el = document.getElementById('current-queue');
    if (!el) return;
    el.style.animation = 'none';
    void el.offsetWidth;
    el.style.animation = 'sc-pulse 0.5s ease 2';
}
