use crate::api::FunctionDeclaration;
use crate::tools::AgentTool;
use async_trait::async_trait;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

fn notes_dir() -> PathBuf {
    if let Ok(cwd) = std::env::current_dir() {
        let dir = cwd.join(".rune").join("notes");
        let _ = fs::create_dir_all(&dir);
        return dir;
    }
    PathBuf::from(".rune/notes")
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

        let mut notes = Vec::new();
        let mut stack = vec![dir.clone()];
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
                        if let Ok(rel) = p.strip_prefix(&dir) {
                            notes.push(rel.to_string_lossy().to_string());
                        } else {
                            notes.push(p.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        notes.sort();
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

        let dir = notes_dir();
        let full_path = dir.join(note_path);

        // Security check against directory traversal
        if let Ok(canonical_dir) = fs::canonicalize(&dir) {
            if let Ok(canonical_target) = fs::canonicalize(&full_path) {
                if !canonical_target.starts_with(&canonical_dir) {
                    return "Error: Access denied (path outside .rune/notes/).".to_string();
                }
            }
        }

        if !full_path.exists() {
            return format!(
                "Error: Rune-note '{}' not found in .rune/notes/.",
                note_path
            );
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

        let dir = notes_dir();
        let full_path = dir.join(note_path);

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if let Ok(canonical_dir) = fs::canonicalize(&dir) {
            if let Some(parent) = full_path.parent() {
                if let Ok(canonical_parent) = fs::canonicalize(parent) {
                    if !canonical_parent.starts_with(&canonical_dir) {
                        return "Error: Access denied (path outside .rune/notes/).".to_string();
                    }
                }
            }
        }

        match fs::write(&full_path, content) {
            Ok(()) => format!("Successfully saved rune-note '{}'.", note_path),
            Err(e) => format!("Error saving rune-note '{}': {}", note_path, e),
        }
    }
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
        let output_file = args.get("output_path")
            .and_then(|v| v.as_str())
            .unwrap_or("rune_notes.html");

        let dir = notes_dir();
        if !dir.exists() {
            return "No rune-notes directory found (.rune/notes/ is empty).".to_string();
        }

        // Collect all markdown notes
        let mut notes: Vec<(String, String)> = Vec::new(); // (relative_path, content)
        let mut stack = vec![dir.clone()];
        while let Some(path) = stack.pop() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        stack.push(p);
                    } else if p.extension().map_or(false, |ext| ext == "md" || ext == "markdown") {
                        if let Ok(rel) = p.strip_prefix(&dir) {
                            let rel_str = rel.to_string_lossy().replace('\\', "/");
                            if let Ok(content) = fs::read_to_string(&p) {
                                notes.push((rel_str, content));
                            }
                        }
                    }
                }
            }
        }

        if notes.is_empty() {
            return "No rune-notes found to render.".to_string();
        }

        notes.sort_by(|a, b| a.0.cmp(&b.0));

        // Generate standalone HTML with Vis.network graph & interactive link lines
        let mut html = String::new();
        html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
        html.push_str("<meta charset=\"UTF-8\">\n");
        html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
        html.push_str("<title>Rune Notes Knowledge Graph</title>\n");
        html.push_str("<script type=\"text/javascript\" src=\"https://unpkg.com/vis-network/standalone/umd/vis-network.min.js\"></script>\n");
        html.push_str("<script src=\"https://cdn.jsdelivr.net/npm/marked/marked.js\"></script>\n");
        html.push_str("<style>\n");
        html.push_str("  :root { --bg: #0f172a; --panel: #1e293b; --border: #334155; --text: #f8fafc; --muted: #94a3b8; --accent: #38bdf8; --accent-hover: #0ea5e9; }\n");
        html.push_str("  body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: var(--bg); color: var(--text); margin: 0; display: flex; height: 100vh; overflow: hidden; }\n");
        html.push_str("  sidebar { width: 340px; background: var(--panel); border-right: 1px solid var(--border); display: flex; flex-direction: column; z-index: 10; }\n");
        html.push_str("  .sidebar-header { padding: 20px; border-bottom: 1px solid var(--border); }\n");
        html.push_str("  .sidebar-header h2 { margin: 0; font-size: 1.25rem; color: var(--accent); }\n");
        html.push_str("  .search-box { width: 100%; padding: 8px 12px; margin-top: 12px; background: var(--bg); border: 1px solid var(--border); color: var(--text); border-radius: 6px; box-sizing: border-box; }\n");
        html.push_str("  .note-list { overflow-y: auto; flex: 1; padding: 12px; list-style: none; margin: 0; }\n");
        html.push_str("  .note-item { padding: 10px 14px; margin-bottom: 4px; border-radius: 6px; cursor: pointer; color: var(--muted); font-size: 0.9rem; transition: all 0.2s; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }\n");
        html.push_str("  .note-item:hover { background: rgba(56, 189, 248, 0.1); color: var(--text); }\n");
        html.push_str("  .note-item.active { background: var(--accent); color: var(--bg); font-weight: 600; }\n");
        html.push_str("  main { flex: 1; display: flex; flex-direction: column; overflow: hidden; background: var(--bg); position: relative; }\n");
        html.push_str("  #graph-view { flex: 1; width: 100%; height: 100%; background: var(--bg); }\n");
        html.push_str("  .reader-pane { position: absolute; top: 0; right: 0; width: 55%; height: 100%; background: var(--panel); border-left: 1px solid var(--border); box-shadow: -5px 0 25px rgba(0,0,0,0.5); display: flex; flex-direction: column; z-index: 20; transform: translateX(100%); transition: transform 0.3s ease; }\n");
        html.push_str("  .reader-pane.open { transform: translateX(0); }\n");
        html.push_str("  .content-header { padding: 16px 24px; border-bottom: 1px solid var(--border); background: var(--panel); font-size: 0.9rem; color: var(--muted); display: flex; justify-content: space-between; align-items: center; }\n");
        html.push_str("  .close-btn { background: none; border: none; color: var(--muted); font-size: 1.25rem; cursor: pointer; }\n");
        html.push_str("  .close-btn:hover { color: var(--text); }\n");
        html.push_str("  .content-body { flex: 1; padding: 32px; overflow-y: auto; line-height: 1.6; color: var(--text); }\n");
        html.push_str("  h1, h2, h3 { color: var(--accent); }\n");
        html.push_str("  code { background: var(--bg); padding: 2px 6px; border-radius: 4px; font-family: monospace; font-size: 0.9em; border: 1px solid var(--border); }\n");
        html.push_str("  pre code { display: block; padding: 16px; overflow-x: auto; border: 1px solid var(--border); }\n");
        html.push_str("  a { color: var(--accent); text-decoration: none; }\n");
        html.push_str("  a:hover { text-decoration: underline; }\n");
        html.push_str("  blockquote { border-left: 4px solid var(--accent); margin: 0; padding-left: 16px; color: var(--muted); }\n");
        html.push_str("  table { width: 100%; border-collapse: collapse; margin: 16px 0; }\n");
        html.push_str("  th, td { border: 1px solid var(--border); padding: 8px 12px; text-align: left; }\n");
        html.push_str("  th { background: var(--panel); }\n");
        html.push_str("  .graph-legend { position: absolute; bottom: 20px; left: 20px; background: rgba(30, 41, 59, 0.9); border: 1px solid var(--border); padding: 12px 16px; border-radius: 8px; font-size: 0.85rem; color: var(--muted); backdrop-filter: blur(4px); pointer-events: none; z-index: 5; }\n");
        html.push_str("  .legend-item { display: flex; align-items: center; margin-bottom: 4px; }\n");
        html.push_str("  .legend-dot { width: 10px; height: 10px; border-radius: 50%; margin-right: 8px; }\n");
        html.push_str("</style>\n");
        html.push_str("</head>\n<body>\n");

        // Sidebar
        html.push_str("<sidebar>\n");
        html.push_str("  <div class=\"sidebar-header\">\n");
        html.push_str("    <h2>⚡ Rune Notes Graph</h2>\n");
        html.push_str("    <input type=\"text\" class=\"search-box\" id=\"search\" placeholder=\"Search notes...\" oninput=\"filterNotes()\">\n");
        html.push_str("  </div>\n");
        html.push_str("  <ul class=\"note-list\" id=\"noteList\">\n");
        for (idx, (path, _)) in notes.iter().enumerate() {
            html.push_str(&format!(
                "    <li class=\"note-item\" onclick=\"openNoteIndex({})\" title=\"{}\">{}</li>\n",
                idx,
                path,
                path
            ));
        }
        html.push_str("  </ul>\n");
        html.push_str("</sidebar>\n");

        // Main Graph Area & Reader Pane
        html.push_str("<main>\n");
        html.push_str("  <div id=\"graph-view\"></div>\n");
        html.push_str("  <div class=\"graph-legend\">\n");
        html.push_str("    <div class=\"legend-item\"><div class=\"legend-dot\" style=\"background: #38bdf8;\"></div>Rune Note Node</div>\n");
        html.push_str("    <div class=\"legend-item\"><div class=\"legend-dot\" style=\"background: #0ea5e9;\"></div>Connected Link Line</div>\n");
        html.push_str("    <div style=\"margin-top: 6px; font-size: 0.75rem; color: #64748b;\">Tip: Click any node to read content & highlight links</div>\n");
        html.push_str("  </div>\n");
        html.push_str("  <div class=\"reader-pane\" id=\"readerPane\">\n");
        html.push_str("    <div class=\"content-header\">\n");
        html.push_str("      <span id=\"contentHeader\">Viewing: </span>\n");
        html.push_str("      <button class=\"close-btn\" onclick=\"closeReader()\">×</button>\n");
        html.push_str("    </div>\n");
        html.push_str("    <div class=\"content-body\" id=\"contentBody\"></div>\n");
        html.push_str("  </div>\n");
        html.push_str("</main>\n");

        // Embedded JSON data and Vis.network setup
        html.push_str("<script>\n");
        html.push_str("  const notesData = {\n");
        for (idx, (path, content)) in notes.iter().enumerate() {
            let escaped = content
                .replace('\\', "\\\\")
                .replace('`', "\\`")
                .replace('$', "\\$");
            html.push_str(&format!("    \"note_{}\": {{ id: \"note_{}\", path: \"{}\", content: `{}` }},\n", idx, idx, path, escaped));
        }
        html.push_str("  };\n");

        html.push_str("  // Build nodes and edges for Vis.network\n");
        html.push_str("  const nodesArray = [];\n");
        html.push_str("  const edgesArray = [];\n");
        html.push_str("  const pathMap = {};\n");

        for (idx, (path, _)) in notes.iter().enumerate() {
            html.push_str(&format!("  pathMap[\"{}\"] = \"note_{}\";\n", path, idx));
            let label = path.split('/').last().unwrap_or(path);
            html.push_str(&format!(
                "  nodesArray.push({{ id: \"note_{}\", label: \"{}\", title: \"{}\", group: \"notes\", color: {{ background: '#1e293b', border: '#38bdf8', highlight: {{ background: '#38bdf8', border: '#0ea5e9' }} }}, font: {{ color: '#f8fafc', size: 14 }}, shape: 'box', margin: 10, borderWidth: 2, borderRadius: 6 }});\n",
                idx, label, path
            ));
        }

        // Parse [[links]] manually in Rust without external regex crate
        for (idx, (_, content)) in notes.iter().enumerate() {
            let mut search_idx = 0;
            while let Some(start) = content[search_idx..].find("[[") {
                let abs_start = search_idx + start + 2;
                if let Some(end) = content[abs_start..].find("]]") {
                    let abs_end = abs_start + end;
                    let link_content = &content[abs_start..abs_end];
                    let target_raw = if let Some(pipe) = link_content.find('|') {
                        link_content[..pipe].trim()
                    } else {
                        link_content.trim()
                    };

                    for (t_idx, (t_path, _)) in notes.iter().enumerate() {
                        if t_path == target_raw || t_path.ends_with(&format!("/{}", target_raw)) || target_raw.ends_with(&format!("/{}", t_path)) {
                            if idx != t_idx {
                                html.push_str(&format!(
                                    "  edgesArray.push({{ from: \"note_{}\", to: \"note_{}\", color: {{ color: '#334155', highlight: '#38bdf8' }}, width: 2, arrows: {{ to: {{ enabled: true, scaleFactor: 0.5 }} }} }});\n",
                                    idx, t_idx
                                ));
                            }
                        }
                    }
                    search_idx = abs_end + 2;
                } else {
                    break;
                }
            }
        }

        html.push_str("  const nodes = new vis.DataSet(nodesArray);\n");
        html.push_str("  const edges = new vis.DataSet(edgesArray);\n");
        html.push_str("  const container = document.getElementById('graph-view');\n");
        html.push_str("  const data = { nodes: nodes, edges: edges };\n");
        html.push_str("  const options = {\n");
        html.push_str("    physics: {\n");
        html.push_str("      barnesHut: {\n");
        html.push_str("        gravitationalConstant: -3000,\n");
        html.push_str("        centralGravity: 0.3,\n");
        html.push_str("        springLength: 120,\n");
        html.push_str("        springConstant: 0.04,\n");
        html.push_str("        damping: 0.09\n");
        html.push_str("      }\n");
        html.push_str("    },\n");
        html.push_str("    interaction: { hover: true, tooltipDelay: 200 },\n");
        html.push_str("    nodes: { shadow: true },\n");
        html.push_str("    edges: { shadow: true, smooth: { type: 'cubicBezier', roundness: 0.2 } }\n");
        html.push_str("  };\n");
        html.push_str("  const network = new vis.Network(container, data, options);\n");

        html.push_str("  network.on(\"click\", function(params) {\n");
        html.push_str("    if (params.nodes.length > 0) {\n");
        html.push_str("      const nodeId = params.nodes[0];\n");
        html.push_str("      selectNote(nodeId);\n");
        html.push_str("    }\n");
        html.push_str("  });\n");

        html.push_str("  function openNoteIndex(idx) {\n");
        html.push_str("    selectNote('note_' + idx);\n");
        html.push_str("    network.selectNodes(['note_' + idx]);\n");
        html.push_str("  }\n");

        html.push_str("  function selectNote(id) {\n");
        html.push_str("    document.querySelectorAll('.note-item').forEach(i => i.classList.remove('active'));\n");
        html.push_str("    for (const item of document.querySelectorAll('.note-item')) {\n");
        html.push_str("      if (item.getAttribute('title') === notesData[id].path) {\n");
        html.push_str("        item.classList.add('active');\n");
        html.push_str("        break;\n");
        html.push_str("      }\n");
        html.push_str("    }\n");
        html.push_str("    document.getElementById('contentHeader').innerText = 'Viewing: ' + notesData[id].path;\n");
        html.push_str("    const note = notesData[id];\n");
        html.push_str("    if (note) {\n");
        html.push_str("      let htmlContent = marked.parse(note.content);\n");
        html.push_str("      htmlContent = htmlContent.replace(/\\[\\[([^\\]\\|]+)(?:\\|([^\\]]+))?\\]\\]/g, (match, p1, p2) => {\n");
        html.push_str("        const title = p2 || p1;\n");
        html.push_str("        return `<a href=\"#\" onclick=\"openNoteByPath('${p1.trim()}'); return false;\">${title}</a>`;\n");
        html.push_str("      });\n");
        html.push_str("      document.getElementById('contentBody').innerHTML = htmlContent;\n");
        html.push_str("      document.getElementById('readerPane').classList.add('open');\n");
        html.push_str("    }\n");
        html.push_str("  }\n");

        html.push_str("  function closeReader() {\n");
        html.push_str("    document.getElementById('readerPane').classList.remove('open');\n");
        html.push_str("    document.querySelectorAll('.note-item').forEach(i => i.classList.remove('active'));\n");
        html.push_str("    network.unselectAll();\n");
        html.push_str("  }\n");

        html.push_str("  function openNoteByPath(path) {\n");
        html.push_str("    for (const [id, note] of Object.entries(notesData)) {\n");
        html.push_str("      if (note.path === path || note.path.endsWith('/' + path) || path.endsWith('/' + note.path)) {\n");
        html.push_str("        selectNote(id);\n");
        html.push_str("        network.selectNodes([id]);\n");
        html.push_str("        return;\n");
        html.push_str("      }\n");
        html.push_str("    }\n");
        html.push_str("  }\n");

        html.push_str("  function filterNotes() {\n");
        html.push_str("    const q = document.getElementById('search').value.toLowerCase();\n");
        html.push_str("    document.querySelectorAll('.note-item').forEach(item => {\n");
        html.push_str("      const text = item.innerText.toLowerCase();\n");
        html.push_str("      item.style.display = text.includes(q) ? 'block' : 'none';\n");
        html.push_str("    });\n");
        html.push_str("  }\n");

        html.push_str("</script>\n");
        html.push_str("</body>\n</html>\n");

        match fs::write(output_file, html) {
            Ok(()) => format!("Successfully rendered {} rune-notes into an interactive Vis.network knowledge graph with connection lines at '{}'.", notes.len(), output_file),
            Err(e) => format!("Error writing HTML output to '{}': {}", output_file, e),
        }
    }
}
