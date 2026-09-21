extends Control
## UI del updater. Reacciona a las señales del UpdaterController.
## No contiene lógica de negocio: solo presentación.

# Los estados viven en UpdaterController.State para no duplicar el enum.

const COLOR_BG := Color("#141623")
const COLOR_PANEL := Color("#1d2133")
const COLOR_ACCENT := Color("#4f8cff")
const COLOR_ERROR := Color("#ff5c7a")
const COLOR_TEXT := Color("#e6e9f5")
const COLOR_MUTED := Color("#8b93ad")

@onready var developer_logo: TextureRect = %DeveloperLogo
@onready var banner_texture: TextureRect = %BannerTexture
@onready var banner_indicators: HBoxContainer = %BannerIndicators
@onready var status_label: Label = %StatusLabel
@onready var version_label: Label = %VersionLabel
@onready var progress_bar: ProgressBar = %ProgressBar
@onready var percent_label: Label = %PercentLabel
@onready var ok_button: Button = %OkButton
@onready var retry_button: Button = %RetryButton

var controller: UpdaterController
var banner_manager: BannerManager
var logger: Node

var _default_logo: Texture2D
var _banner_textures: Array = []
var _banner_index: int = 0


func setup(ctrl: UpdaterController, banners: BannerManager, log_node: Node) -> void:
	controller = ctrl
	banner_manager = banners
	logger = log_node


func _ready() -> void:
	_apply_theme()
	_default_logo = developer_logo.texture

	ok_button.pressed.connect(_on_ok_pressed)
	retry_button.pressed.connect(_on_retry_pressed)
	ok_button.visible = false
	retry_button.visible = false
	progress_bar.value = 0
	percent_label.text = "0%"
	status_label.text = "Starting updater..."
	_set_version_text("", "")

	if controller != null:
		controller.state_changed.connect(_on_state_changed)
		controller.progress_changed.connect(_on_progress_changed)
		controller.versions_changed.connect(_set_version_text)
	if banner_manager != null:
		banner_manager.banners_partial_ready.connect(_on_banners_ready)
		banner_manager.banners_ready.connect(_on_banners_ready)
		banner_manager.banner_changed.connect(_on_banner_changed)


func _apply_theme() -> void:
	var style := StyleBoxFlat.new()
	style.bg_color = COLOR_PANEL
	style.corner_radius_top_left = 10
	style.corner_radius_top_right = 10
	style.corner_radius_bottom_left = 10
	style.corner_radius_bottom_right = 10
	style.content_margin_left = 16
	style.content_margin_right = 16
	style.content_margin_top = 12
	style.content_margin_bottom = 12
	if %InfoPanel is PanelContainer:
		%InfoPanel.add_theme_stylebox_override("panel", style)
	status_label.add_theme_color_override("font_color", COLOR_TEXT)
	version_label.add_theme_color_override("font_color", COLOR_MUTED)
	percent_label.add_theme_color_override("font_color", COLOR_MUTED)


func _on_state_changed(state: int, data: Dictionary) -> void:
	var message := String(data.get("message", ""))
	status_label.text = message

	match state:
		UpdaterController.State.STARTING, UpdaterController.State.LOADING_MANIFEST, UpdaterController.State.LOADING_BANNERS:
			progress_bar.value = 0
			percent_label.text = "0%"
			ok_button.visible = false
			retry_button.visible = false
			status_label.add_theme_color_override("font_color", COLOR_TEXT)
		UpdaterController.State.DOWNLOADING, UpdaterController.State.VERIFYING, UpdaterController.State.EXTRACTING, UpdaterController.State.INSTALLING:
			ok_button.visible = false
			retry_button.visible = false
		UpdaterController.State.COMPLETED:
			progress_bar.value = 100
			percent_label.text = "100%"
			ok_button.visible = true
			retry_button.visible = false
			status_label.add_theme_color_override("font_color", COLOR_TEXT)
			ok_button.grab_focus()
			if bool(data.get("up_to_date", false)):
				status_label.text = "✓ " + message
			else:
				status_label.text = "✓ " + message
				_set_version_text(String(data.get("current", "")), String(data.get("target", "")))
		UpdaterController.State.ERROR:
			ok_button.visible = false
			retry_button.visible = true
			status_label.add_theme_color_override("font_color", COLOR_ERROR)
			status_label.text = "⚠ " + message
			retry_button.grab_focus()
		UpdaterController.State.PREVIEW:
			progress_bar.value = 0
			percent_label.text = ""
			ok_button.visible = false
			retry_button.visible = false
			status_label.add_theme_color_override("font_color", COLOR_MUTED)
			version_label.text = "Preview de interfaz"


func _on_progress_changed(ratio: float, _received: int, _total: int) -> void:
	progress_bar.value = ratio * 100.0
	percent_label.text = "%d%%" % int(round(ratio * 100.0))


func _set_version_text(current: String, target: String) -> void:
	var left := current if not current.is_empty() else "?"
	if target.is_empty():
		version_label.text = "Versión actual %s" % left
	else:
		version_label.text = "%s → %s" % [left, target]


func _on_banners_ready(textures: Array) -> void:
	_banner_textures = textures.duplicate()
	_banner_index = 0
	_rebuild_banner_indicators()
	if _banner_textures.is_empty():
		return
	banner_texture.texture = _banner_textures[0]
	banner_texture.visible = true


func _on_banner_changed(texture: Texture2D, index: int) -> void:
	if texture != null:
		banner_texture.texture = texture
		_banner_index = index
		_update_banner_indicators()


func _rebuild_banner_indicators() -> void:
	for child in banner_indicators.get_children():
		banner_indicators.remove_child(child)
		child.queue_free()
	for i in _banner_textures.size():
		var button := Button.new()
		button.flat = true
		button.focus_mode = Control.FOCUS_NONE
		button.custom_minimum_size = Vector2(24, 24)
		button.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
		button.tooltip_text = "Mostrar banner %d" % (i + 1)
		button.pressed.connect(_on_banner_indicator_pressed.bind(i))
		banner_indicators.add_child(button)
	_update_banner_indicators()


func _update_banner_indicators() -> void:
	var buttons := banner_indicators.get_children()
	for i in buttons.size():
		var button := buttons[i] as Button
		button.text = "●" if i == _banner_index else "○"
		button.add_theme_color_override("font_color", COLOR_ACCENT if i == _banner_index else COLOR_MUTED)


func _on_banner_indicator_pressed(index: int) -> void:
	if banner_manager != null:
		banner_manager.select_banner(index)


func _on_ok_pressed() -> void:
	var relauncher := AppRelauncher.new()
	relauncher.relaunch(controller.args.target_root, controller.args.executable, logger)
	get_tree().quit()


func _on_retry_pressed() -> void:
	get_tree().quit()
