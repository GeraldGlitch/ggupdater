class_name ProcessWaiter
extends RefCounted
## Espera a que termine el proceso indicado por --pid.
## Si no hay PID, espera un intervalo corto de cortesía antes de continuar.

const DEFAULT_WAIT_SECONDS := 3.0
const POLL_INTERVAL := 0.5


## Espera de forma cooperativa (sin bloquear el hilo principal) a que el PID muera.
## Devuelve true si el proceso ya no está (o no había PID).
func wait_for_exit(pid: int, logger: Node = null, max_seconds: float = 120.0) -> bool:
	if pid <= 0:
		_log(logger, "info", "Sin --pid; esperando %.1fs de cortesía." % DEFAULT_WAIT_SECONDS)
		await _delay(DEFAULT_WAIT_SECONDS)
		return true

	if not is_process_alive(pid):
		_log(logger, "info", "El proceso %d ya no está activo." % pid)
		return true

	_log(logger, "info", "Esperando cierre del proceso %d..." % pid)
	var started := Time.get_ticks_msec()
	var waited := 0.0
	while waited < max_seconds:
		if not is_process_alive(pid):
			_log(logger, "info", "Proceso %d cerrado tras %.1fs." % [pid, waited])
			return true
		await _delay(POLL_INTERVAL)
		waited = float(Time.get_ticks_msec() - started) / 1000.0

	_log(logger, "warn", "Timeout esperando al proceso %d; se continúa igualmente." % pid)
	return false


## Pausa que cede el hilo: permite que la UI siga dibujándose durante la espera.
func _delay(seconds: float) -> void:
	var tree := Engine.get_main_loop() as SceneTree
	if tree == null:
		OS.delay_msec(int(seconds * 1000))
		return
	await tree.create_timer(seconds).timeout


## Comprueba si un PID sigue vivo sin depender de librerías externas.
static func is_process_alive(pid: int) -> bool:
	if pid <= 0:
		return false
	if OS.get_name() == "Windows":
		return _alive_windows(pid)
	return _alive_unix(pid)


static func _alive_unix(pid: int) -> bool:
	var path := "/proc/%d" % pid
	if DirAccess.dir_exists_absolute(path):
		return true
	# Fallback multiplataforma vía kill -0.
	return OS.execute("kill", ["-0", str(pid)], [], true) == 0


static func _alive_windows(pid: int) -> bool:
	var output: Array = []
	var code := OS.execute("tasklist", ["/FI", "PID eq %d" % pid, "/NH"], output, true)
	if code != 0:
		return false
	var text := "\n".join(output)
	return text.contains(str(pid))


func _log(logger: Node, level: String, message: String) -> void:
	if logger != null and logger.has_method(level):
		logger.call(level, message)
