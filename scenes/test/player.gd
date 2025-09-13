extends CharacterBody2D

@export var SPEED := 100

var direction:= Vector2.ZERO

@onready var animation_tree: AnimationTree = $AnimationTree

func _ready():
	print("=== Local Player Initialized ===")

func _physics_process(_delta: float) -> void:
	direction = Input.get_vector("ui_left", "ui_right", "ui_up", "ui_down").normalized()
	
	if direction != Vector2.ZERO:
		var animation_direction := Vector2(direction.x, -direction.y)
		update_blend_positions(animation_direction)
	
	if GameConfig.game_mode == GameConfig.GameMode.SINGLE_PLAYER:
		# Handle full local movement
		velocity = direction * SPEED
		move_and_slide()
	else:
		# Multiplayer: Send input to server, position updated by broadcasts
		var game_manager = get_node_or_null("../../GameManager")
		if game_manager:
			game_manager.send_movement_input()

func update_blend_positions(direction_vector: Vector2) -> void:
	animation_tree.set("parameters/StateMachine/MoveState/RunState/blend_position", direction_vector)
	animation_tree.set("parameters/StateMachine/MoveState/StandState/blend_position", direction_vector)
