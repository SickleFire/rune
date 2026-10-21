use crate::api::FunctionDeclaration;
use crate::tools::AgentTool;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

fn notes_dir() -> PathBuf {
    if let Ok(cwd) = std::env::current_dir() {
        let dir = cwd.join(".rune").join("notes");
        let _ = fs::create_dir_all(&dir);
        return dir;
    }
    PathBuf::from(".rune/notes")
}

/// Only allow plain relative paths (no `..`, no absolute paths, no drive prefixes).
fn is_safe_rel(p: &str) -> bool {
    let path = Path::new(p);
    !p.is_empty()
        && path
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Recursively collect all markdown files under `dir`, returning (relative_path, absolute_path).
fn collect_note_files(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p
                    .extension()
                    .map_or(false, |ext| ext == "md" || ext == "markdown")
                {
                    let rel = match p.strip_prefix(dir) {
                        Ok(r) => r.to_string_lossy().replace('\\', "/"),
                        Err(_) => p.to_string_lossy().replace('\\', "/"),
                    };
                    out.push((rel, p));
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

pub struct ListRuneNotesTool;

#[async_trait]
impl AgentTool for ListRuneNotesTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "list_rune_notes".to_string(),
            description: "List all available rune-notes (knowledge nodes) in .rune/notes/"
                .to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {},
                "required": []
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, _args: serde_json::Value) -> String {
        let dir = notes_dir();
        if !dir.exists() {
            return "No rune-notes directory found (.rune/notes/ is empty).".to_string();
        }

        let notes: Vec<String> = collect_note_files(&dir)
            .into_iter()
            .map(|(rel, _)| rel)
            .collect();

        if notes.is_empty() {
            return "No rune-notes (.md files) found in .rune/notes/.".to_string();
        }

        format!("Available rune-notes:\n- {}", notes.join("\n- "))
    }
}

pub struct ReadRuneNoteTool;

#[async_trait]
impl AgentTool for ReadRuneNoteTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "read_rune_note".to_string(),
            description: "Read the content of a specific rune-note (knowledge node) by its relative path or ID (e.g. 'overview.md' or 'src/api.rs.md').".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "note_path": {
                        "type": "STRING",
                        "description": "The path or name of the note inside .rune/notes/ (e.g., 'overview.md' or 'concepts/godot.md')."
                    }
                },
                "required": ["note_path"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, args: serde_json::Value) -> String {
        let note_path = match args.get("note_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return "Error: 'note_path' parameter is required.".to_string(),
        };

        if !is_safe_rel(note_path) {
            return "Error: Access denied (path outside .rune/notes/).".to_string();
        }

        let dir = notes_dir();
        let full_path = dir.join(note_path);

        if !full_path.exists() {
            return format!(
                "Error: Rune-note '{}' not found in .rune/notes/.",
                note_path
            );
        }

        // Defense in depth: resolve symlinks and make sure we are still inside the notes dir.
        match (fs::canonicalize(&dir), fs::canonicalize(&full_path)) {
            (Ok(canonical_dir), Ok(canonical_target)) => {
                if !canonical_target.starts_with(&canonical_dir) {
                    return "Error: Access denied (path outside .rune/notes/).".to_string();
                }
            }
            _ => return "Error: Could not resolve note path.".to_string(),
        }

        match fs::read_to_string(&full_path) {
            Ok(content) => format!("--- Rune Note: {} ---\n\n{}", note_path, content),
            Err(e) => format!("Error reading rune-note '{}': {}", note_path, e),
        }
    }
}

pub struct UpsertRuneNoteTool;

#[async_trait]
impl AgentTool for UpsertRuneNoteTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "upsert_rune_note".to_string(),
            description: "Create or update a rune-note (knowledge node) in .rune/notes/ with Markdown content.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "note_path": {
                        "type": "STRING",
                        "description": "The path or name of the note (e.g., 'overview.md' or 'src/api.rs.md')."
                    },
                    "content": {
                        "type": "STRING",
                        "description": "The full Markdown content of the note (including frontmatter if desired)."
                    }
                },
                "required": ["note_path", "content"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn execute(&self, args: serde_json::Value) -> String {
        let note_path = match args.get("note_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return "Error: 'note_path' parameter is required.".to_string(),
        };
        let content = match args.get("content").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return "Error: 'content' parameter is required.".to_string(),
        };

        // Validate BEFORE creating any directories.
        if !is_safe_rel(note_path) {
            return "Error: Access denied (path outside .rune/notes/).".to_string();
        }

        let dir = notes_dir();
        let full_path = dir.join(note_path);

        if let Some(parent) = full_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return format!("Error creating directories for '{}': {}", note_path, e);
            }
            if let (Ok(canonical_dir), Ok(canonical_parent)) =
                (fs::canonicalize(&dir), fs::canonicalize(parent))
            {
                if !canonical_parent.starts_with(&canonical_dir) {
                    return "Error: Access denied (path outside .rune/notes/).".to_string();
                }
            }
        }

        match fs::write(&full_path, content) {
            Ok(()) => format!("Successfully saved rune-note '{}'.", note_path),
            Err(e) => format!("Error saving rune-note '{}': {}", note_path, e),
        }
    }
}

pub struct SearchRuneNotesTool;

#[async_trait]
impl AgentTool for SearchRuneNotesTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "search_notes".to_string(),
            description: "Search across all rune-notes (.rune/notes/) by query string matching file paths or content.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "query": {
                        "type": "STRING",
                        "description": "The search term or keyword to match against note content or paths."
                    },
                    "case_sensitive": {
                        "type": "BOOLEAN",
                        "description": "Whether the search should be case-sensitive (default: false)."
                    }
                },
                "required": ["query"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, args: serde_json::Value) -> String {
        let query = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) if !q.is_empty() => q,
            _ => return "Error: 'query' parameter is required and cannot be empty.".to_string(),
        };
        let case_sensitive = args
            .get("case_sensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let dir = notes_dir();
        if !dir.exists() {
            return "No rune-notes directory found (.rune/notes/ is empty).".to_string();
        }

        let note_files = collect_note_files(&dir);
        if note_files.is_empty() {
            return "No rune-notes (.md files) found in .rune/notes/.".to_string();
        }

        let mut results = Vec::new();

        for (rel_path, full_path) in note_files {
            let content = match fs::read_to_string(&full_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let path_matches = if case_sensitive {
                rel_path.contains(query)
            } else {
                rel_path.to_lowercase().contains(&query.to_lowercase())
            };

            let mut matching_lines = Vec::new();
            for (idx, line) in content.lines().enumerate() {
                let line_matches = if case_sensitive {
                    line.contains(query)
                } else {
                    line.to_lowercase().contains(&query.to_lowercase())
                };

                if line_matches {
                    matching_lines.push((idx + 1, line.to_string()));
                    if matching_lines.len() >= 5 {
                        break; // limit matching lines per file for conciseness
                    }
                }
            }

            if path_matches || !matching_lines.is_empty() {
                results.push((rel_path, path_matches, matching_lines));
            }
        }

        if results.is_empty() {
            return format!("No rune-notes found matching query '{}'.", query);
        }

        let mut output = format!("Found matching rune-notes for query '{}':\n", query);
        for (rel, path_match, lines) in results {
            output.push_str(&format!(
                "\n- **{}**{}",
                rel,
                if path_match { " (path match)" } else { "" }
            ));
            for (line_num, line_text) in lines {
                output.push_str(&format!("\n  - Line {}: `{}`", line_num, line_text.trim()));
            }
        }
        output
    }
}

const GRAPH_TEMPLATE: &str = r###"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Rune Notes Knowledge Graph</title>
<script src="https://unpkg.com/vis-network@9.1.9/standalone/umd/vis-network.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/marked@12.0.2/marked.min.js"></script>
<style>
  :root { --bg: #000000; --panel: #050505; --border: #1a1a1a; --text: #ededed; --muted: #737373; --accent: #fafafa; }
  * { scrollbar-width: thin; scrollbar-color: #262626 transparent; }
  body { font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: var(--bg); color: var(--text); margin: 0; display: flex; height: 100vh; overflow: hidden; -webkit-font-smoothing: antialiased; }
  .sidebar { width: 320px; background: linear-gradient(180deg, #0e0e0e 0%, #050505 45%, #000000 100%); border-right: 1px solid var(--border); display: flex; flex-direction: column; z-index: 10; }
  .sidebar-header { padding: 28px 20px 18px; border-bottom: 1px solid var(--border); }
  .sidebar-header h2 { margin: 0; font-size: 0.75rem; font-weight: 600; letter-spacing: 0.22em; text-transform: uppercase; background: linear-gradient(90deg, #ffffff 0%, #5c5c5c 100%); -webkit-background-clip: text; background-clip: text; color: transparent; }
  .search-box { width: 100%; padding: 9px 12px; margin-top: 16px; background: linear-gradient(180deg, #0b0b0b, #000000); border: 1px solid var(--border); color: var(--text); border-radius: 6px; box-sizing: border-box; font-size: 0.85rem; outline: none; transition: border-color 0.2s; }
  .search-box::placeholder { color: #525252; }
  .search-box:focus { border-color: #404040; }
  .note-list { overflow-y: auto; flex: 1; padding: 12px; list-style: none; margin: 0; }
  .note-item { padding: 9px 12px; margin-bottom: 2px; border-radius: 6px; cursor: pointer; color: var(--muted); font-size: 0.85rem; border-left: 2px solid transparent; transition: all 0.2s; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .note-item:hover { background: linear-gradient(90deg, #151515 0%, rgba(0,0,0,0) 100%); color: var(--text); }
  .note-item.active { background: linear-gradient(90deg, #262626 0%, rgba(0,0,0,0) 100%); border-left-color: #fafafa; color: #ffffff; font-weight: 500; }
  main { flex: 1; display: flex; flex-direction: column; overflow: hidden; background: var(--bg); position: relative; }
  #graph-view { flex: 1; width: 100%; height: 100%; background: radial-gradient(ellipse at 50% 40%, #0d0d0d 0%, #050505 40%, #000000 80%); }
  .reader-pane { position: absolute; top: 0; right: 0; width: 55%; height: 100%; background: linear-gradient(180deg, #0c0c0c 0%, #040404 40%, #000000 100%); border-left: 1px solid var(--border); box-shadow: -20px 0 60px rgba(0,0,0,0.9); display: flex; flex-direction: column; z-index: 20; transform: translateX(100%); transition: transform 0.3s ease; }
  .reader-pane.open { transform: translateX(0); }
  .content-header { padding: 18px 28px; border-bottom: 1px solid var(--border); background: linear-gradient(90deg, #111111 0%, #000000 100%); font-size: 0.75rem; letter-spacing: 0.05em; color: var(--muted); display: flex; justify-content: space-between; align-items: center; }
  .close-btn { background: none; border: none; color: var(--muted); font-size: 1.25rem; line-height: 1; cursor: pointer; transition: color 0.2s; }
  .close-btn:hover { color: #ffffff; }
  .content-body { flex: 1; padding: 36px 40px; overflow-y: auto; line-height: 1.7; font-size: 0.95rem; color: #d4d4d4; }
  .content-body pre.plain { white-space: pre-wrap; word-wrap: break-word; font-family: monospace; }
  .content-body h1, .content-body h2, .content-body h3 { color: #ffffff; font-weight: 600; letter-spacing: -0.01em; }
  .content-body h1 { background: linear-gradient(90deg, #ffffff 0%, #6b6b6b 100%); -webkit-background-clip: text; background-clip: text; color: transparent; }
  code { background: #0a0a0a; padding: 2px 6px; border-radius: 4px; font-family: 'SF Mono', Menlo, Consolas, monospace; font-size: 0.85em; border: 1px solid var(--border); color: #e5e5e5; }
  pre code { display: block; padding: 16px; overflow-x: auto; background: linear-gradient(180deg, #0b0b0b, #000000); }
  a { color: #ffffff; text-decoration: none; border-bottom: 1px solid #404040; transition: border-color 0.2s; }
  a:hover { border-bottom-color: #ffffff; }
  blockquote { border-left: 2px solid #404040; margin: 0; padding-left: 16px; color: var(--muted); }
  hr { border: none; height: 1px; background: linear-gradient(90deg, #2a2a2a, rgba(0,0,0,0)); }
  table { width: 100%; border-collapse: collapse; margin: 16px 0; }
  th, td { border: 1px solid var(--border); padding: 8px 12px; text-align: left; }
  th { background: linear-gradient(180deg, #121212, #050505); }
  .graph-legend { position: absolute; bottom: 24px; left: 24px; background: linear-gradient(180deg, rgba(14,14,14,0.9), rgba(0,0,0,0.9)); border: 1px solid var(--border); padding: 12px 16px; border-radius: 8px; font-size: 0.75rem; letter-spacing: 0.03em; color: var(--muted); backdrop-filter: blur(6px); pointer-events: none; z-index: 5; }
  .legend-item { display: flex; align-items: center; margin-bottom: 4px; }
  .legend-dot { width: 8px; height: 8px; border-radius: 50%; margin-right: 10px; }
</style>
</head>
<body>
<aside class="sidebar">
  <div class="sidebar-header">
    <h2>Rune Notes</h2>
    <input type="text" class="search-box" id="search" placeholder="Search notes..." oninput="filterNotes()">
  </div>
  <ul class="note-list" id="noteList">
__SIDEBAR__
  </ul>
</aside>
<main>
  <div id="graph-view"></div>
  <div class="graph-legend">
    <div class="legend-item"><div class="legend-dot" style="background: linear-gradient(135deg, #ffffff, #737373);"></div>Note</div>
    <div class="legend-item"><div class="legend-dot" style="background: linear-gradient(135deg, #525252, #1a1a1a);"></div>Link</div>
    <div style="margin-top: 6px; color: #525252;">Click a node to read it</div>
  </div>
  <div class="reader-pane" id="readerPane">
    <div class="content-header">
      <span id="contentHeader">Viewing: </span>
      <button class="close-btn" onclick="closeReader()">×</button>
    </div>
    <div class="content-body" id="contentBody"></div>
  </div>
</main>
<script>
  const rawNodes = __NODES__;
  const rawEdges = __EDGES__;
  const notesData = __NOTES__;

  const nodes = new vis.DataSet(rawNodes.map(n => ({
    id: n.id,
    label: n.label,
    title: n.title,
    group: 'notes',
    shape: 'box',
    margin: 10,
    borderWidth: 2,
    shapeProperties: { borderRadius: 6 },
    color: {
      background: '#000000',
      border: '#404040',
      highlight: { background: '#171717', border: '#fafafa' },
      hover: { background: '#0f0f0f', border: '#8a8a8a' }
    },
    font: { color: '#e5e5e5', size: 13, face: 'Inter, -apple-system, Segoe UI, Roboto, sans-serif' }
  })));
  const edges = new vis.DataSet(rawEdges.map(e => ({
    from: e.from,
    to: e.to,
    color: { color: '#2a2a2a', highlight: '#fafafa', hover: '#737373' },
    width: 1,
    arrows: { to: { enabled: true, scaleFactor: 0.4 } }
  })));

  const container = document.getElementById('graph-view');
  const options = {
    physics: {
      barnesHut: {
        gravitationalConstant: -3000,
        centralGravity: 0.3,
        springLength: 120,
        springConstant: 0.04,
        damping: 0.09
      }
    },
    interaction: { hover: true, tooltipDelay: 200 },
    nodes: { shadow: false },
    edges: { shadow: false, smooth: { type: 'cubicBezier', roundness: 0.2 } }
  };
  const network = new vis.Network(container, { nodes: nodes, edges: edges }, options);

  network.on('click', function (params) {
    if (params.nodes.length > 0) {
      selectNote(params.nodes[0]);
    }
  });

  function openNoteIndex(idx) {
    selectNote('note_' + idx);
    network.selectNodes(['note_' + idx]);
  }

  function selectNote(id) {
    const note = notesData[id];
    if (!note) return;

    document.querySelectorAll('.note-item').forEach(i => i.classList.remove('active'));
    for (const item of document.querySelectorAll('.note-item')) {
      if (item.getAttribute('data-path') === note.path) {
        item.classList.add('active');
        break;
      }
    }

    document.getElementById('contentHeader').textContent = 'Viewing: ' + note.path;
    const body = document.getElementById('contentBody');

    try {
      if (window.marked) {
        // Strip YAML frontmatter so it doesn't render as a stray heading/hr.
        const md = note.content.replace(/^---\r?\n[\s\S]*?\r?\n---\r?\n?/, '');
        const parse = marked.parse ? marked.parse.bind(marked) : marked;
        let htmlContent = parse(md);
        htmlContent = htmlContent.replace(/\[\[([^\]\|]+)(?:\|([^\]]+))?\]\]/g, function (m, p1, p2) {
          const label = (p2 || p1).trim();
          return '<a href="#" class="wikilink" data-target="' + encodeURIComponent(p1.trim()) + '">' + label + '</a>';
        });
        body.innerHTML = htmlContent;
      } else {
        const pre = document.createElement('pre');
        pre.className = 'plain';
        pre.textContent = note.content;
        body.innerHTML = '';
        body.appendChild(pre);
      }
    } catch (err) {
      console.error('Failed to render note', err);
      const pre = document.createElement('pre');
      pre.className = 'plain';
      pre.textContent = note.content;
      body.innerHTML = '';
      body.appendChild(pre);
    }

    body.scrollTop = 0;
    document.getElementById('readerPane').classList.add('open');
  }

  document.getElementById('contentBody').addEventListener('click', function (e) {
    const a = e.target.closest('a.wikilink');
    if (!a) return;
    e.preventDefault();
    openNoteByPath(decodeURIComponent(a.getAttribute('data-target')));
  });

  function closeReader() {
    document.getElementById('readerPane').classList.remove('open');
    document.querySelectorAll('.note-item').forEach(i => i.classList.remove('active'));
    network.unselectAll();
  }

  function openNoteByPath(path) {
    for (const [id, note] of Object.entries(notesData)) {
      if (note.path === path || note.path.endsWith('/' + path) || path.endsWith('/' + note.path)) {
        selectNote(id);
        network.selectNodes([id]);
        return;
      }
    }
  }

  function filterNotes() {
    const q = document.getElementById('search').value.toLowerCase();
    document.querySelectorAll('.note-item').forEach(item => {
      const text = item.textContent.toLowerCase();
      item.style.display = text.includes(q) ? 'block' : 'none';
    });
  }
</script>
</body>
</html>
"###;

/// Serialize JSON so it is safe to embed inside a <script> block.
fn json_for_script(value: &serde_json::Value) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "null".to_string())
        .replace("</", "<\\/")
        .replace("<!--", "<\\!--")
}

pub struct RenderRuneNotesTool;

#[async_trait]
impl AgentTool for RenderRuneNotesTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "render_rune_notes".to_string(),
            description: "Render all rune-notes (.rune/notes/) into an interactive Vis.network knowledge graph with node link lines and a markdown reader pane.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "output_path": {
                        "type": "STRING",
                        "description": "Optional output HTML file path (defaults to 'rune_notes.html')."
                    }
                },
                "required": []
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn execute(&self, args: serde_json::Value) -> String {
        let output_file = args
            .get("output_path")
            .and_then(|v| v.as_str())
            .unwrap_or("rune_notes.html");

        let dir = notes_dir();
        if !dir.exists() {
            return "No rune-notes directory found (.rune/notes/ is empty).".to_string();
        }

        // Collect all markdown notes as (relative_path, content), already sorted by path.
        let notes: Vec<(String, String)> = collect_note_files(&dir)
            .into_iter()
            .filter_map(|(rel, abs)| fs::read_to_string(&abs).ok().map(|c| (rel, c)))
            .collect();

        if notes.is_empty() {
            return "No rune-notes found to render.".to_string();
        }

        // Sidebar list items (HTML-escaped).
        let mut sidebar = String::new();
        for (idx, (path, _)) in notes.iter().enumerate() {
            let esc = html_escape(path);
            sidebar.push_str(&format!(
                "    <li class=\"note-item\" onclick=\"openNoteIndex({})\" data-path=\"{}\" title=\"{}\">{}</li>\n",
                idx, esc, esc, esc
            ));
        }

        // Notes data, nodes, and edges, all built as JSON.
        let mut notes_map = serde_json::Map::new();
        let mut nodes_arr = Vec::new();
        for (idx, (path, content)) in notes.iter().enumerate() {
            let id = format!("note_{}", idx);
            notes_map.insert(
                id.clone(),
                json!({ "id": id, "path": path, "content": content }),
            );
            let label = path.rsplit('/').next().unwrap_or(path);
            nodes_arr.push(json!({ "id": id, "label": label, "title": path }));
        }

        // Parse [[links]] without an external regex crate.
        let mut seen: HashSet<(usize, usize)> = HashSet::new();
        let mut edges_arr = Vec::new();
        for (idx, (_, content)) in notes.iter().enumerate() {
            let mut search_idx = 0;
            while let Some(start) = content[search_idx..].find("[[") {
                let abs_start = search_idx + start + 2;
                let Some(end) = content[abs_start..].find("]]") else {
                    break;
                };
                let abs_end = abs_start + end;
                let link_content = &content[abs_start..abs_end];
                let target_raw = match link_content.find('|') {
                    Some(pipe) => link_content[..pipe].trim(),
                    None => link_content.trim(),
                };

                if !target_raw.is_empty() {
                    for (t_idx, (t_path, _)) in notes.iter().enumerate() {
                        let matches = t_path == target_raw
                            || t_path.ends_with(&format!("/{}", target_raw))
                            || target_raw.ends_with(&format!("/{}", t_path))
                            // allow links written without the .md extension
                            || t_path == &format!("{}.md", target_raw)
                            || t_path.ends_with(&format!("/{}.md", target_raw));
                        if matches && idx != t_idx && seen.insert((idx, t_idx)) {
                            edges_arr.push(json!({
                                "from": format!("note_{}", idx),
                                "to": format!("note_{}", t_idx)
                            }));
                        }
                    }
                }
                search_idx = abs_end + 2;
            }
        }

        let nodes_json = json_for_script(&serde_json::Value::Array(nodes_arr));
        let edges_json = json_for_script(&serde_json::Value::Array(edges_arr));
        let notes_json = json_for_script(&serde_json::Value::Object(notes_map));

        // Replace the notes data LAST so note content can never be touched by other replacements.
        let html = GRAPH_TEMPLATE
            .replace("__SIDEBAR__", &sidebar)
            .replace("__NODES__", &nodes_json)
            .replace("__EDGES__", &edges_json)
            .replace("__NOTES__", &notes_json);

        match fs::write(output_file, html) {
            Ok(()) => format!(
                "Successfully rendered {} rune-notes into an interactive Vis.network knowledge graph with connection lines at '{}'.",
                notes.len(),
                output_file
            ),
            Err(e) => format!("Error writing HTML output to '{}': {}", output_file, e),
        }
    }
}
