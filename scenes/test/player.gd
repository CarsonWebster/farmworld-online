extends CharacterBody2D

@export var SPEED := 100

var direction:= Vector2.ZERO

@onready var animation_tree: AnimationTree = $AnimationTree

func _ready():
	print("=== Local Player Initialized ===")

func _physics_process(_delta: float) -> void:
	# Handle local movement (for visual feedback)
	direction = Input.get_vector("ui_left", "ui_right", "ui_up", "ui_down").normalized()
	
	if direction != Vector2.ZERO:
		var animation_direction := Vector2(direction.x, -direction.y)
		update_blend_positions(animation_direction)
	
	velocity = direction * SPEED
	move_and_slide()

	# Send movement to server (delegate to GameManager)
	var game_manager = get_node_or_null("../../GameManager")
	if game_manager:
		game_manager.send_movement_input()

func update_blend_positions(direction_vector: Vector2) -> void:
	animation_tree.set("parameters/StateMachine/MoveState/RunState/blend_position", direction_vector)
	animation_tree.set("parameters/StateMachine/MoveState/StandState/blend_position", direction_vector)
