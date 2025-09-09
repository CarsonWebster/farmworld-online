extends CharacterBody2D

@export var SPEED := 100

func _physics_process(delta: float) -> void:
	var direction = Input.get_vector("ui_left", "ui_right", "ui_up", "ui_down").normalized()
	velocity = direction * SPEED
	move_and_slide()
