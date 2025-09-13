extends Control

func _ready():
	$ColorRect/SingleplayerButton.connect("pressed", _on_singleplayer_pressed)
	$ColorRect/MultiplayerButton.connect("pressed", _on_multiplayer_pressed)

func _on_singleplayer_pressed():
	GameConfig.game_mode = GameConfig.GameMode.SINGLE_PLAYER
	print("Game Mode: ", "Single Player" if GameConfig.game_mode == GameConfig.GameMode.SINGLE_PLAYER else "Multiplayer")
	get_tree().change_scene_to_file("res://scenes/test/test_world.tscn")

func _on_multiplayer_pressed():
	GameConfig.game_mode = GameConfig.GameMode.MULTIPLAYER
	print("Game Mode: ", "Single Player" if GameConfig.game_mode == GameConfig.GameMode.SINGLE_PLAYER else "Multiplayer")
	get_tree().change_scene_to_file("res://scenes/test/test_world.tscn")
