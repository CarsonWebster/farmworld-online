extends CharacterBody2D

@export var SPEED := 100

func _ready():
	# Disable input processing for remote players
	set_process_input(false)
	set_physics_process(false)
	print("=== Remote Player Initialized ===")

# Position updates are handled by PlayerManager via direct position setting
# This player has no input control or physics - it's purely visual
