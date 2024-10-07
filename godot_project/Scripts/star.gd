extends Area2D

func _process(delta: float) -> void:
	rotation += 0.5 * delta


func collect():
	$CPUParticles2D2.call_deferred("set_emitting", false)
	$Sprite2D.call_deferred("set_visible", false)
	$CollisionShape2D.call_deferred("set_disabled", true)
	$CPUParticles2D.call_deferred("set_emitting", true)
	
	$AudioStreamPlayer.play()
