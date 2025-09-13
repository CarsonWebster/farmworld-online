extends Node

signal player_joined(player_id: String, position: Vector2)
signal player_left(player_id: String)
signal player_moved(player_id: String, position: Vector2)

var websocket: WebSocketPeer
var connected := false
var local_player_id := ""
var last_direction := Vector2.ZERO

func _ready():
	print("=== GameManager Starting ===")
	if GameConfig.game_mode == GameConfig.GameMode.MULTIPLAYER:
		connect_to_server()
	else:
		# Single-player: Simulate local join
		print("🎮 Single-player mode: Simulating local player join")
		var fake_player_id = "local_player"
		var start_position = Vector2(365, 175)  # Match server spawn position
		local_player_id = fake_player_id
		emit_signal("player_joined", fake_player_id, start_position)

func connect_to_server():
	websocket = WebSocketPeer.new()
	var error = websocket.connect_to_url("ws://127.0.0.1:9001")
	if error != OK:
		print("❌ Failed to connect to server: ", error)
		return
	print("🔌 Attempting to connect to: ws://127.0.0.1:9001")

func _physics_process(delta):
	if websocket:
		websocket.poll()
		var state = websocket.get_ready_state()

		if state == WebSocketPeer.STATE_OPEN:
			if not connected:
				connected = true
				print("✅ Connected to server successfully")
				# Send join message after connection
				get_tree().create_timer(0.1).timeout.connect(func(): send_join_message())

			# Process incoming messages
			while websocket.get_available_packet_count() > 0:
				var packet = websocket.get_packet()
				var message = packet.get_string_from_utf8()
				handle_server_message(message)

		elif state == WebSocketPeer.STATE_CONNECTING:
			if connected:
				connected = false
				print("🔄 Reconnecting to server...")

		elif state == WebSocketPeer.STATE_CLOSED:
			if connected:
				connected = false
				print("❌ Disconnected from server")
				var code = websocket.get_close_code()
				var reason = websocket.get_close_reason()
				print("   Close code: ", code, ", Reason: ", reason)

func send_join_message():
	if websocket.get_ready_state() == WebSocketPeer.STATE_OPEN:
		var join_data = {
			"action": "Join"
		}
		var json_string = JSON.stringify(join_data)
		websocket.send_text(json_string)
		print("📤 Sent JOIN message: ", json_string)
	else:
		print("⚠️  Cannot send join message - WebSocket not connected")

func send_movement_input():
	if GameConfig.game_mode == GameConfig.GameMode.MULTIPLAYER:
		if websocket.get_ready_state() == WebSocketPeer.STATE_OPEN:
			var direction = Input.get_vector("ui_left", "ui_right", "ui_up", "ui_down")

			# Only send if direction changed significantly
			if direction.distance_to(last_direction) > 0.01:
				last_direction = direction

				var move_data = {
					"action": "Move",
					"data": {
						"dx": direction.x,
						"dy": direction.y
					}
				}
				var json_string = JSON.stringify(move_data)
				websocket.send_text(json_string)
				print("📤 Sent MOVE message: ", json_string)
		else:
			print("⚠️  Cannot send movement - WebSocket not connected")
	# In single-player, movement is handled locally by player.gd

func handle_server_message(message: String):
	#print("📥 Received from server: ", message)

	var json = JSON.new()
	var error = json.parse(message)

	if error == OK:
		var data = json.get_data()
		var event_type = data.get("event", "unknown")

		match event_type:
			"PlayerJoined":
				var event_data = data.get("data", {})
				var player_id = event_data.get("player_id", "")
				var x = event_data.get("x", 0.0)
				var y = event_data.get("y", 0.0)
				local_player_id = player_id  # Store local player ID
				emit_signal("player_joined", player_id, Vector2(x, y))
				print("🎮 PLAYER JOINED - ID: ", player_id, " at (", x, ", ", y, ")")

			"PlayerState":
				var event_data = data.get("data", {})
				var players = event_data.get("players", [])
				#print("📊 PLAYER STATE UPDATE - ", players.size(), " players:")
				for player in players:
					var pid = player.get("player_id", "")
					var x = player.get("x", 0.0)
					var y = player.get("y", 0.0)
					emit_signal("player_moved", pid, Vector2(x, y))
					#var is_me = pid == local_player_id
					#var marker = "👤" if is_me else "👥"
					#print("   ", marker, " Player ", pid.substr(0, 8), "... at (", x, ", ", y, ")")

			"PlayerLeft":
				var event_data = data.get("data", {})
				var pid = event_data.get("player_id", "")
				emit_signal("player_left", pid)
				print("👋 PLAYER LEFT - ID: ", pid)

			_:
				print("❓ UNKNOWN EVENT: ", event_type, " - Full data: ", data)
	else:
		print("❌ Failed to parse server message: ", message)
		print("   JSON Parse Error: ", error)

func _notification(what):
	if what == NOTIFICATION_WM_CLOSE_REQUEST:
		print("🛑 Window close requested - disconnecting from server...")
		if websocket and websocket.get_ready_state() == WebSocketPeer.STATE_OPEN:
			websocket.close()
		else:
			print("⚠️  WebSocket already closed or not connected")

func _exit_tree():
	print("🏁 GameManager exiting - cleaning up WebSocket...")
	if websocket:
		if websocket.get_ready_state() == WebSocketPeer.STATE_OPEN:
			websocket.close()
		print("✅ WebSocket cleanup complete")
