extends Node
## Punto de entrada. Crea logger, parsea argumentos, configura controller/banners/UI.

const UpdaterScene := preload("res://ui/updater_ui.tscn")

var logger: Node
var args: LaunchArgs
var banner_manager: BannerManager
var controller: UpdaterController


func _ready() -> void:
	logger = preload("res://scripts/logger.gd").new()
	logger.name = "Logger"
	add_child(logger)

	args = LaunchArgs.parse()
	var target_version := ""
	logger.configure(args.app_id, args.current_version, target_version)
	logger.info("Argumentos: " + JSON.stringify(args.to_dictionary()))

	banner_manager = BannerManager.new()
	banner_manager.name = "BannerManager"
	add_child(banner_manager)
	banner_manager.setup(args.app_id, logger)

	controller = UpdaterController.new()
	controller.name = "UpdaterController"
	add_child(controller)
	controller.setup(args, logger, banner_manager)

	var ui := UpdaterScene.instantiate()
	ui.setup(controller, banner_manager, logger)
	add_child(ui)

	controller.start()
