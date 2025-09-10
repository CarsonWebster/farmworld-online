extends Node

@export var player_scene: PackedScene
@export var other_player_scene: PackedScene

var players := {} # player_id -> player_instance
var local_player_id := ""
var has_spawned_local := false

func _ready():
	print("=== PlayerManager Starting ===")
	print("DEBUG: has_spawned_local = ", has_spawned_local, ", local_player_id = '", local_player_id, "'")
	# Connect to GameManager signals
	var game_manager = get_node("../GameManager")
	if game_manager:
		game_manager.connect("player_joined", _on_player_joined)
		game_manager.connect("player_left", _on_player_left)
		game_manager.connect("player_moved", _on_player_moved)
		print("✅ Connected to GameManager signals")
	else:
		print("❌ Could not find GameManager node")

func spawn_local_player(player_id: String, position: Vector2):
	var player = player_scene.instantiate()
	player.position = position
	add_child(player)
	players[player_id] = player
	print("🎮 Spawned LOCAL player: ", player_id, " at ", position)

func _on_player_joined(player_id: String, position: Vector2):
	print("📥 Player joined: ", player_id, " at ", position)
	print("   has_spawned_local: ", has_spawned_local, ", local_player_id: '", local_player_id, "'")

	if player_id == local_player_id:
		# Already spawned as local player, ignore
		return

	if not has_spawned_local:
		# First player to join is always the local player
		spawn_local_player(player_id, position)
		has_spawned_local = true
		local_player_id = player_id
		print("🎯 First player set as LOCAL: ", player_id)
	else:
		spawn_other_player(player_id, position)

func spawn_other_player(player_id: String, position: Vector2):
	var other_player = other_player_scene.instantiate()
	other_player.position = position
	add_child(other_player)
	players[player_id] = other_player
	print("👥 Spawned REMOTE player: ", player_id, " at ", position)

func _on_player_left(player_id: String):
	if players.has(player_id):
		var player_instance = players[player_id]
		if player_instance:
			player_instance.queue_free()
		players.erase(player_id)
		print("👋 Removed player: ", player_id)
	else:
		print("⚠️  Tried to remove unknown player: ", player_id)

func _on_player_moved(player_id: String, position: Vector2):
	if players.has(player_id):
		var player_instance = players[player_id]
		if player_instance:
			# Smooth interpolation for remote players
			player_instance.position = player_instance.position.lerp(position, 0.3)
	else:
		print("⚠️  Tried to move unknown player: ", player_id)
