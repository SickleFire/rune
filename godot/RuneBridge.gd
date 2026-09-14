@tool
extends EditorPlugin

var server: TCPServer = TCPServer.new()
const PORT: int = 8089
var active_peers: Array[StreamPeerTCP] = []
var server_running: bool = false

func _enter_tree() -> void:
	var err = server.listen(PORT)
	if err == OK:
		server_running = true
		print("[RuneBridge] Godot Editor HTTP server listening on http://localhost:%d/" % PORT)
		set_process(true)
	else:
		server_running = false
		printerr("[RuneBridge] Failed to start HTTP server on port %d: error %d" % [PORT, err])
		set_process(false)

func _exit_tree() -> void:
	server.stop()
	for peer in active_peers:
		peer.disconnect_from_host()
	active_peers.clear()
	server_running = false
	print("[RuneBridge] Godot Editor HTTP server stopped.")

func _process(_delta: float) -> void:
	if not server_running:
		return

	if not server.is_connection_available():
		var peers_to_remove: Array[StreamPeerTCP] = []
		for peer in active_peers:
			peer.poll()
			var status = peer.get_status()
			if status == StreamPeerTCP.STATUS_CONNECTED:
				if peer.get_available_bytes() > 0:
					handle_client(peer)
					peers_to_remove.append(peer)
			elif status != StreamPeerTCP.STATUS_CONNECTING:
				peers_to_remove.append(peer)
		
		for p in peers_to_remove:
			active_peers.erase(p)
		return

	var connection = server.take_connection()
	if connection:
		active_peers.append(connection)

func handle_client(peer: StreamPeerTCP) -> void:
	var request_text = ""
	while peer.get_available_bytes() > 0:
		var chunk = peer.get_data(peer.get_available_bytes())
		if chunk[0] == OK:
			request_text += chunk[1].get_string_from_utf8()
		else:
			break

	if request_text.is_empty():
		return

	var lines = request_text.split("\r\n")
	if lines.size() == 0:
		return

	var request_line = lines[0].split(" ")
	if request_line.size() < 2:
		return

	var method = request_line[0]
	var full_path = request_line[1]

	var path = full_path
	var query_params = {}
	var q_idx = full_path.find("?")
	if q_idx != -1:
		path = full_path.substr(0, q_idx)
		var query_str = full_path.substr(q_idx + 1)
		var pairs = query_str.split("&")
		for pair in pairs:
			var kv = pair.split("=")
			if kv.size() == 2:
				query_params[kv[0]] = kv[1].uri_decode()

	var body = ""
	if method == "POST":
		var body_idx = request_text.find("\r\n\r\n")
		if body_idx != -1:
			body = request_text.substr(body_idx + 4)

	var response_data = {"error": "Endpoint not found"}
	var status_code = 404

	if path == "/scene/inspect" and method == "GET":
		response_data = get_scene_hierarchy()
		status_code = 200
	elif path == "/node/inspect-properties" and method == "GET":
		var node_path = query_params.get("nodePath", "")
		if node_path.is_empty():
			response_data = {"error": "Missing 'nodePath' query parameter"}
			status_code = 400
		else:
			response_data = inspect_node_properties(node_path)
			status_code = 200
	elif path == "/node/create" and method == "POST":
		var json_data = JSON.parse_string(body)
		if json_data and typeof(json_data) == TYPE_DICTIONARY:
			response_data = create_node(json_data)
			status_code = 200
		else:
			response_data = {"status": "error", "message": "Invalid JSON body"}
			status_code = 400
	elif path == "/node/destroy" and method == "POST":
		var json_data = JSON.parse_string(body)
		if json_data and typeof(json_data) == TYPE_DICTIONARY:
			response_data = destroy_node(json_data)
			status_code = 200
		else:
			response_data = {"status": "error", "message": "Invalid JSON body"}
			status_code = 400
	elif path == "/editor/play-mode" and method == "POST":
		response_data = {"status": "success", "message": "Godot play mode command received"}
		status_code = 200

	send_http_response(peer, status_code, JSON.stringify(response_data))

func send_http_response(peer: StreamPeerTCP, status: int, json_body: String) -> void:
	var status_text = "OK" if status == 200 else ("Bad Request" if status == 400 else "Not Found")
	var response = "HTTP/1.1 %d %s\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: %d\r\nConnection: close\r\n\r\n%s" % [status, status_text, json_body.to_utf8_buffer().size(), json_body]
	peer.put_data(response.to_utf8_buffer())
	peer.disconnect_from_host()

func get_scene_hierarchy() -> Dictionary:
	var edited_scene = EditorInterface.get_edited_scene_root()
	if not edited_scene:
		return {"sceneName": null, "rootNodes": []}
	
	return {
		"sceneName": edited_scene.name,
		"rootNodes": [serialize_node(edited_scene)]
	}

func serialize_node(node: Node) -> Dictionary:
	var children = []
	for child in node.get_children():
		children.append(serialize_node(child))
	
	return {
		"name": node.name,
		"type": node.get_class(),
		"path": str(node.get_path()),
		"children": children
	}

func inspect_node_properties(node_path: String) -> Dictionary:
	var edited_scene = EditorInterface.get_edited_scene_root()
	var node: Node = null
	if edited_scene:
		if str(edited_scene.get_path()) == node_path or node_path == "." or node_path == str(edited_scene.name):
			node = edited_scene
		else:
			node = edited_scene.get_node_or_null(node_path)
			if not node:
				node = edited_scene.find_child(node_path, true, false)
	
	if not node:
		return {"error": "Node not found"}
	
	var props = []
	for p in node.get_property_list():
		var prop_name = p["name"]
		var val = node.get(prop_name)
		props.append({
			"name": prop_name,
			"type": p["type"],
			"value": str(val) if val != null else "null"
		})
	
	return {
		"nodeName": node.name,
		"nodeType": node.get_class(),
		"nodePath": str(node.get_path()),
		"properties": props
	}

func create_node(payload: Dictionary) -> Dictionary:
	var parent_path = payload.get("parentPath", "")
	var node_name = payload.get("nodeName", "NewNode")
	var node_type = payload.get("nodeType", "Node")
	
	var edited_scene = EditorInterface.get_edited_scene_root()
	if not edited_scene:
		return {"status": "error", "message": "No active edited scene"}
	
	var parent: Node = edited_scene
	if not parent_path.is_empty() and parent_path != "." and parent_path != str(edited_scene.name):
		parent = edited_scene.get_node_or_null(parent_path)
		if not parent:
			parent = edited_scene.find_child(parent_path, true, false)
	
	if not parent:
		parent = edited_scene
	
	var new_node: Node = null
	if ClassDB.class_exists(node_type):
		var obj = ClassDB.instantiate(node_type)
		if obj is Node:
			new_node = obj
	
	if not new_node:
		new_node = Node.new()
	
	new_node.name = node_name
	parent.add_child(new_node)
	new_node.owner = edited_scene
	
	# Mark scene as modified in editor
	EditorInterface.get_resource_filesystem().scan()
	
	return {
		"status": "success",
		"nodeName": new_node.name,
        "path": str(new_node.get_path())
	}

func destroy_node(payload: Dictionary) -> Dictionary:
	var node_path = payload.get("plugin.cfg", "") # or nodePath
	var path_to_remove = payload.get("nodePath", "")
	var edited_scene = EditorInterface.get_edited_scene_root()
	if not edited_scene:
		return {"status": "error", "message": "No active edited scene"}
	
	var node: Node = edited_scene.get_node_or_null(path_to_remove)
	if not node:
		node = edited_scene.find_child(path_to_remove, true, false)
	
	if not node:
		return {"status": "error", "message": "Node not found"}
	
	var name = node.name
	node.free()
	
	return {"status": "success", "destroyed": name}
