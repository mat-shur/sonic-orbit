extends Area2D

@export var joystick: VirtualJoystick
@export var speed: float = 500
@export var thrust: float = 2000
@export var rotation_speed: float = 5.0
@export var max_speed: float = 600
@export var static_speed: float = 500
@export var boost_strength: float = 3000
@export var boost_duration: float = 4.50

const PLAYER_SHADOW = preload("res://Scenes/player_shadow.tscn")
const FLOATING_TEXT = preload("res://Scenes/floating_text.tscn")
const CoinScene = preload("res://Scenes/coin_animation.tscn")

var velocity := Vector2.ZERO
var is_boosting := true
var invincible := false
var count_of_coins := 0

@onready var map = $"../.."
@onready var invincible_bar = $UI/InvincibleBar/Bar
@onready var invincible_bar_timer = $UI/InvincibleBar/BarTimer
@onready var arc = $Arc
@onready var rocket = $Rocket

var lives = 1;

var rocket_type;

var start_bonus = 25000

func _ready() -> void:
	map.get_node("PlayerData").load_game()
	rocket_type = map.get_node("PlayerData").type_rocket

	print("From player:", rocket_type)

	var textures = [
		preload("res://Assets/rockets/0.png"),
		preload("res://Assets/rockets/1.png"),
		preload("res://Assets/rockets/2.png"),
		preload("res://Assets/rockets/3.png")
	]

	rotation = 0
	lives = 1
	$Arc2.visible = false
		
	$Rocket.texture = textures[rocket_type]

	rotation_speed = 5 + map.get_node("PlayerData").upg_rotation * 0.2
	boost_duration = 4.5 + map.get_node("PlayerData").upg_boost_duration * 0.25
	boost_strength = 3000 + map.get_node("PlayerData").upg_boost_speed * 100
	static_speed = 400 + map.get_node("PlayerData").upg_speed * 25

	if int(rocket_type) == 1:
		lives = 2
		$Arc2.visible = true


func _process(delta: float) -> void:
	if not map.started:
		return 

	update_shadow_timer()
	update_score()
	update_max_speed()
	update_movement(delta)
	apply_boost(delta)

func update_shadow_timer() -> void:
	$ShadowTimer.wait_time = max(0.5 / (1 + abs(velocity.y) / 200), 0.01)

func update_score() -> void:
	$UI/Score.text = str(start_bonus + round(-position.y/100))

func update_max_speed() -> void:
	max_speed = static_speed + round(-position.y/500)

func update_movement(delta: float) -> void:
	var move_vector = Input.get_vector("ui_left", "ui_right", "ui_up", "ui_down")
	if move_vector.length() > 0:
		var target_rotation = atan2(move_vector.y, move_vector.x) + PI / 2
		rotation = lerp_angle(rotation, target_rotation, rotation_speed * delta)

	velocity = velocity.limit_length(max_speed)
	global_position += velocity * delta

func apply_boost(delta: float) -> void:
	if is_boosting:
		var direction = Vector2.from_angle(rotation - PI / 2)
		velocity += direction * boost_strength * delta

func _on_bar_timer_timeout() -> void:
	if invincible:
		update_invincibility()

func update_invincibility() -> void:
	if invincible_bar.value > 0:
		invincible_bar.value -= 5
	else:
		end_invincibility()

func end_invincibility() -> void:
	static_speed -= 500  # invincible_speed_bonus
	invincible_bar_timer.stop()
	$UI/InvincibleBar.visible = false
	
	var tween = create_tween()
	tween.tween_property(arc, "modulate:a", 0, 1)
	tween.tween_property(rocket, "modulate", Color.WHITE, 1)
	
	await tween.finished
	invincible = false
	arc.visible = false

func _on_shadow_timer_timeout() -> void:
	var shadow = PLAYER_SHADOW.instantiate()
	shadow.texture = $Rocket.texture
	get_parent().add_child(shadow)
	shadow.global_transform = global_transform
	
	if invincible:
		shadow.modulate = Color("#2890ef")

func _on_area_entered(area: Area2D) -> void:
	if area.is_in_group("coin"):
		collect_coin(area)
	elif area.is_in_group("star"):
		activate_invincibility(area)
	elif (area.is_in_group("meteorite") and not invincible) or (area.is_in_group("dead_line")):
		if lives > 1 and not area.is_in_group("dead_line"):
			lives -= 1
			var tween = create_tween()
			tween.tween_property(area, "modulate:a", 0, 0.3)
			tween.tween_property(area, "visible", false, 0.4)
			
			$Die2.play()
			$Arc2.visible = false
			
			return
			
		$Die.play()
		map.get_node("Menu").play()
		
		map.started = false
		get_tree().paused = true
		
		$dead_screen.visible = true
		$UI/DeadControl.visible = true
		$UI/DeadControl/Control/Score.text = $UI/Score.text
		$UI/DeadControl/Control/Coins.text = $UI/Coins.text
		$UI/DeadControl/Control/Cost.text = "You will get " + str(int($UI/Score.text) + (10 * int($UI/Coins.text))) + " Orbitals!"
		
		$UI/DeadControl/Control/Plus.text = "+" + str(int($UI/Coins.text) * 10)
		
		var tween = create_tween()
		tween.set_parallel(true)
		tween.tween_property($dead_screen, "scale", Vector2(3.5, 3.5), 0.8)
		tween.tween_property($UI/DeadControl/Control, "modulate:a", 1, 0.6)
		
		await tween.finished
		tween = create_tween()
		tween.set_parallel(true)
		
		$UI/DeadControl/Control/Plus.visible = true
		$UI/DeadControl/Control/Coins.visible = false
		$UI/DeadControl/Control/AudioStreamPlayer.play()
		$UI/DeadControl/Control/CPUParticles2D2.restart()
		
		tween.tween_method(set_label_text, int($UI/Score.text), int($UI/Score.text) + (10 * int($UI/Coins.text)), 1)
		tween.tween_property($UI/DeadControl/Control/Plus, "modulate:a", 0, 2)
		
		#restart_game()
func set_label_text(value: int):
	$UI/DeadControl/Control/Score.text = str(value)

func restart_game() -> void:
	RenderingServer.set_default_clear_color(Color.WHITE)
	get_tree().call_deferred("reload_current_scene")

func collect_coin(area: Area2D) -> void:
	var floating_text = FLOATING_TEXT.instantiate()
	if rocket_type == 3:
		floating_text.text = "+2"
		count_of_coins += 2
	else:
		floating_text.text = "+1"
		count_of_coins += 1
	floating_text.global_position = area.global_position - Vector2(0, 50)
	get_parent().add_child(floating_text)

	
	$UI/Coins.text = str(count_of_coins)
	area.collect()

func activate_invincibility(area: Area2D) -> void:
	if not invincible:
		area.collect()
		invincible = true
		invincible_bar.value = 100
		static_speed += 500  # invincible_speed_bonus
		invincible_bar_timer.start()
		$UI/InvincibleBar.visible = true
		arc.visible = true

		var tween = create_tween()
		tween.set_parallel(true)
		tween.tween_property(arc, "modulate:a", 1, 1)
		tween.tween_property(rocket, "modulate", Color("#2890ef"), 1)


func _on_skip_pressed() -> void:
	RenderingServer.set_default_clear_color(Color.WHITE)
	get_tree().paused = false
	get_tree().call_deferred("reload_current_scene")
	
