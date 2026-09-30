import React, { useMemo, useState } from 'react'
import { createRoot } from 'react-dom/client'
import { Activity, ArrowUpRight, BarChart3, Braces, ChevronDown, CircleHelp, Database, FileCode2, Gauge, Hash, Layers3, Play, Plus, Search, Settings2, Sparkles, Terminal, Zap } from 'lucide-react'
import './styles.css'

const tables = [
  { name: 'events', rows: '24,892', size: '1.8 MB', icon: Activity },
  { name: 'sessions', rows: '4,120', size: '420 KB', icon: Layers3 },
  { name: 'accounts', rows: '1,804', size: '196 KB', icon: Database },
]
const seedRows = [
  ['evt_9f20', 'page_view', 'acme_72', '2025-08-29 11:42:08', 'mobile'],
  ['evt_9f1c', 'checkout_start', 'acme_72', '2025-08-29 11:41:52', 'desktop'],
  ['evt_9f18', 'page_view', 'north_11', '2025-08-29 11:40:31', 'mobile'],
  ['evt_9f17', 'signup', 'terra_04', '2025-08-29 11:39:06', 'desktop'],
  ['evt_9f12', 'api_request', 'acme_72', '2025-08-29 11:38:44', 'server'],
]

function App() {
  const [activeTable, setActiveTable] = useState('events')
  const [query, setQuery] = useState('SELECT * FROM events\nWHERE created_at > now() - interval \'1 day\'\nORDER BY created_at DESC\nLIMIT 50;')
  const [ran, setRan] = useState(false)
  const [filter, setFilter] = useState('')
  const rows = useMemo(() => seedRows.filter(row => row.join(' ').toLowerCase().includes(filter.toLowerCase())), [filter])
  return <div className="app-shell">
    <aside className="sidebar">
      <div className="brand"><div className="brand-mark"><Zap size={16} fill="currentColor" /></div><span>Lite Corza<span className="brand-db">DB</span></span><span className="beta">BETA</span></div>
      <div className="workspace-switch"><div className="workspace-dot">C</div><div><strong>Corza workspace</strong><small>local / sandbox.db</small></div><ChevronDown size={15} /></div>
      <nav><p className="nav-label">WORKSPACE</p><button className="nav-item active"><Terminal size={16}/> Query editor <kbd>⌘ K</kbd></button><button className="nav-item"><BarChart3 size={16}/> Insights</button><button className="nav-item"><Gauge size={16}/> Benchmarks <span className="nav-dot"></span></button><p className="nav-label tables-label">TABLES <Plus size={14}/></p>{tables.map(t => { const Icon=t.icon; return <button key={t.name} onClick={()=>setActiveTable(t.name)} className={'nav-item table-item '+(activeTable===t.name?'selected':'')}><Icon size={15}/><span>{t.name}</span><em>{t.rows}</em></button> })}</nav>
      <div className="sidebar-bottom"><button className="nav-item"><Settings2 size={16}/> Settings</button><button className="nav-item"><CircleHelp size={16}/> Docs & shortcuts</button><div className="engine-card"><div className="engine-top"><span className="live-dot"></span><span>ENGINE ONLINE</span><span className="engine-version">v0.1.0</span></div><strong>12.4 MB</strong><small>memory footprint <span>↓ 82% vs SQLite*</span></small></div><div className="profile"><div className="avatar">AM</div><div><strong>Alex Morgan</strong><small>Personal plan</small></div><ChevronDown size={14}/></div></div>
    </aside>
    <main className="main-content">
      <header className="topbar"><div className="breadcrumbs"><span>Corza workspace</span><span>/</span><strong>Query editor</strong></div><div className="top-actions"><span className="shortcut"><Search size={14}/> Search <kbd>⌘ P</kbd></span><button className="icon-button"><Braces size={16}/></button><button className="icon-button"><CircleHelp size={16}/></button><div className="status-pill"><span></span> Local only</div></div></header>
      <section className="content">
        <div className="page-heading"><div><div className="eyebrow"><Sparkles size={13}/> LIGHTWEIGHT BY DEFAULT</div><h1>Query editor</h1><p>Fast, private, and remarkably small. Your data never leaves this device.</p></div><button className="new-query"><Plus size={16}/> New query <kbd>⌘ ↵</kbd></button></div>
        <div className="stats-row"><Stat label="QUERY LATENCY" value={ran?'1.8':'2.4'} unit="ms" hint="p95 · last 100 queries" up="↓ 24%" good/><Stat label="MEMORY USAGE" value="12.4" unit="MB" hint="current process" up="↓ 82%" good/><Stat label="DATABASE SIZE" value="2.4" unit="MB" hint="31,816 total rows" up="↑ 4.2%"/><Stat label="UPTIME" value="14" unit="d" hint="since last restart" up="100%" good/></div>
        <div className="editor-panel panel"><div className="panel-toolbar"><div className="tab-title"><FileCode2 size={16}/> scratch-01.sql <span className="unsaved"></span></div><div className="toolbar-actions"><button className="ghost-button">Format</button><button className="run-button" onClick={()=>setRan(true)}><Play size={14} fill="currentColor"/> Run query <kbd>⌘ ↵</kbd></button></div></div><div className="editor-body"><div className="line-numbers">1<br/>2<br/>3<br/>4</div><textarea value={query} onChange={e=>{setQuery(e.target.value);setRan(false)}} spellCheck="false" aria-label="SQL query editor"/><div className="editor-hint"><span><span className="key">⌘</span> + <span className="key">↵</span> to run</span><span>SQL · UTF-8</span></div></div></div>
        <div className="results-heading"><div><h2>Results <span>{ran?'50 rows':'5 rows'}</span></h2><p>Query completed in <strong>{ran?'1.8':'2.4'} ms</strong> · {activeTable} · read-only</p></div><div className="result-actions"><button className="ghost-button">Explain</button><button className="ghost-button">Export CSV <ArrowUpRight size={13}/></button></div></div>
        <div className="results-panel panel"><div className="table-tools"><div className="search-input"><Search size={14}/><input value={filter} onChange={e=>setFilter(e.target.value)} placeholder="Filter results..."/></div><span className="result-meta"><span className="green-dot"></span> Live result</span></div><div className="data-table"><div className="data-row header"><span>#</span><span>event_id <ChevronDown size={12}/></span><span>event_type</span><span>account_id</span><span>created_at <ChevronDown size={12}/></span><span>source</span></div>{rows.map((row,i)=><div className="data-row" key={row[0]}><span className="row-num">{String(i+1).padStart(2,'0')}</span>{row.map((cell,j)=><span className={j===1?'tag-cell':''} key={cell}>{j===1?<span className={'event-tag '+cell.replace('_','-')}>{cell}</span>:cell}</span>)}</div>)}</div><div className="table-footer"><span>Showing {rows.length} of 24,892 rows</span><span>Page 1 of 498</span></div></div>
        <footer><span><span className="green-dot"></span> Synced locally</span><span>SQLite compatible syntax <span className="footer-sep">·</span> Rust engine <span className="footer-sep">·</span> No telemetry</span></footer>
      </section>
    </main>
  </div>
}
function Stat({label,value,unit,hint,up,good}) { return <div className="stat-card"><div className="stat-label">{label}</div><div className="stat-value">{value}<small>{unit}</small></div><div className="stat-bottom"><span>{hint}</span><b className={good?'good':''}>{up}</b></div></div> }
createRoot(document.getElementById('root')).render(<App />)
