extends Control
## UI del updater. Reacciona a las señales del UpdaterController.
## No contiene lógica de negocio: solo presentación.

const STATE_STARTING := 0
const STATE_LOADING_MANIFEST := 1
const STATE_LOADING_BANNERS := 2
const STATE_DOWNLOADING := 3
const STATE_VERIFYING := 4
const STATE_EXTRACTING := 5
const STATE_INSTALLING := 6
const STATE_COMPLETED := 7
const STATE_ERROR := 8
const STATE_PREVIEW := 9

const COLOR_BG := Color("#141623")
const COLOR_PANEL := Color("#1d2133")
const COLOR_ACCENT := Color("#4f8cff")
const COLOR_ERROR := Color("#ff5c7a")
const COLOR_TEXT := Color("#e6e9f5")
const COLOR_MUTED := Color("#8b93ad")

@onready var developer_logo: TextureRect = %DeveloperLogo
@onready var banner_texture: TextureRect = %BannerTexture
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
		STATE_STARTING, STATE_LOADING_MANIFEST, STATE_LOADING_BANNERS:
			progress_bar.value = 0
			percent_label.text = "0%"
			ok_button.visible = false
			retry_button.visible = false
			status_label.add_theme_color_override("font_color", COLOR_TEXT)
		STATE_DOWNLOADING, STATE_VERIFYING, STATE_EXTRACTING, STATE_INSTALLING:
			ok_button.visible = false
			retry_button.visible = false
		STATE_COMPLETED:
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
		STATE_ERROR:
			ok_button.visible = false
			retry_button.visible = true
			status_label.add_theme_color_override("font_color", COLOR_ERROR)
			status_label.text = "⚠ " + message
			retry_button.grab_focus()
		STATE_PREVIEW:
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
	if not textures.is_empty():
		banner_texture.texture = textures[0]
		banner_texture.visible = true


func _on_banner_changed(texture: Texture2D) -> void:
	if texture != null:
		banner_texture.texture = texture


func _on_ok_pressed() -> void:
	var relauncher := AppRelauncher.new()
	relauncher.relaunch(controller.args.target_root, controller.args.executable, logger)
	get_tree().quit()


func _on_retry_pressed() -> void:
	get_tree().quit()
