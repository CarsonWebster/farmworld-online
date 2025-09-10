extends Control

func _ready():
	$ColorRect/JoinButton.connect("pressed", _on_join_pressed)

func _on_join_pressed():
	get_tree().change_scene_to_file("res://scenes/test/test_world.tscn")
