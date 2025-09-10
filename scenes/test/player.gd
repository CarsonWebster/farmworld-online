extends CharacterBody2D

@export var SPEED := 100

func _ready():
	print("=== Local Player Initialized ===")

func _physics_process(delta: float) -> void:
	# Handle local movement (for visual feedback)
	var direction = Input.get_vector("ui_left", "ui_right", "ui_up", "ui_down").normalized()
	velocity = direction * SPEED
	move_and_slide()

	# Send movement to server (delegate to GameManager)
	var game_manager = get_node_or_null("../../GameManager")
	if game_manager:
		game_manager.send_movement_input()
