extends Node

enum GameMode { SINGLE_PLAYER, MULTIPLAYER }

var game_mode: GameMode = GameMode.MULTIPLAYER  # Default to multiplayer

func _ready():
	print("=== GameConfig Initialized ===")
