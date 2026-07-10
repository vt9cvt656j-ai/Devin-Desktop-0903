use std::path::{Path, PathBuf};
use tokio::fs;

fn tpl(s: &str, name: &str) -> String {
    s.replace("{{NAME}}", name)
}

// ── Phaser 2D Platformer ────────────────────────────────────────────

const PHASER_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{{NAME}}</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{background:#1a1a2e;display:flex;justify-content:center;align-items:center;min-height:100vh;overflow:hidden}
canvas{border-radius:8px;box-shadow:0 0 40px rgba(0,0,0,.5)}
</style>
</head>
<body>
<script src="https://cdn.jsdelivr.net/npm/phaser@3/dist/phaser.min.js"></script>
<script src="game.js"></script>
</body>
</html>"##;

const PHASER_JS: &str = r##"// {{NAME}} — Phaser 2D Platformer
// Arrow keys to move & jump, collect all coins to win

const W = 800, H = 600;
const config = {
  type: Phaser.AUTO, width: W, height: H,
  backgroundColor: '#16213e',
  physics: { default: 'arcade', arcade: { gravity: { y: 600 }, debug: false } },
  scene: { preload, create, update }
};
const game = new Phaser.Game(config);

let player, cursors, platforms, coins, enemies;
let score = 0, scoreText, msgText;

function preload() {}

function create() {
  // ── platforms ──
  platforms = this.physics.add.staticGroup();
  addPlat(this, 400, 580, 800, 28, 0x4a4e69);  // ground
  addPlat(this, 600, 440, 180, 14, 0x7b2cbf);
  addPlat(this, 60,  350, 140, 14, 0x7b2cbf);
  addPlat(this, 400, 290, 200, 14, 0x7b2cbf);
  addPlat(this, 740, 210, 100, 14, 0x7b2cbf);
  addPlat(this, 200, 170, 140, 14, 0x7b2cbf);
  addPlat(this, 540, 110, 120, 14, 0x7b2cbf);

  // ── player ──
  const pg = this.add.rectangle(100, 500, 26, 34, 0x2ec4b6);
  player = this.physics.add.existing(pg);
  player.body.setCollideWorldBounds(true).setBounce(0.1);

  // ── coins ──
  coins = this.physics.add.group();
  [[600,400],[60,310],[400,250],[740,170],[200,130],[540,70],[300,540],[700,540]].forEach(([x,y]) => {
    const c = this.add.circle(x, y, 7, 0xffd700);
    coins.add(c);
    c.body.setAllowGravity(false);
    this.tweens.add({ targets: c, y: y - 6, duration: 800, yoyo: true, repeat: -1, ease: 'Sine.easeInOut' });
  });

  // ── enemies (patrol) ──
  enemies = this.physics.add.group();
  [[500, 420, 140], [300, 270, 120]].forEach(([x, y, range]) => {
    const e = this.add.rectangle(x, y, 20, 20, 0xff6b6b);
    enemies.add(e);
    e.body.setAllowGravity(false);
    this.tweens.add({ targets: e, x: x + range, duration: 2000, yoyo: true, repeat: -1, ease: 'Linear' });
  });

  // ── collisions ──
  this.physics.add.collider(player, platforms);
  this.physics.add.overlap(player, coins, (_, c) => {
    c.destroy();
    score += 10;
    scoreText.setText('Score: ' + score);
    if (coins.countActive() === 0) showMsg(this, '🎉 You Win!');
  });
  this.physics.add.overlap(player, enemies, () => {
    if (msgText) return;
    player.body.setVelocity(0);
    showMsg(this, '💀 Game Over — R to restart');
    this.input.keyboard.once('keydown-R', () => { score = 0; this.scene.restart(); msgText = null; });
  });

  // ── UI ──
  cursors = this.input.keyboard.createCursorKeys();
  scoreText = this.add.text(16, 16, 'Score: 0', { fontSize: '18px', fill: '#e0e0e0', fontFamily: 'monospace' });
  this.add.text(W/2, 14, '{{NAME}}', { fontSize: '12px', fill: '#555', fontFamily: 'monospace' }).setOrigin(0.5, 0);
}

function update() {
  if (msgText) return;
  const v = 180;
  player.body.setVelocityX(cursors.left.isDown ? -v : cursors.right.isDown ? v : 0);
  if (cursors.up.isDown && player.body.touching.down) player.body.setVelocityY(-400);
}

function addPlat(scene, x, y, w, h, color) {
  const r = scene.add.rectangle(x, y, w, h, color);
  platforms.add(r);
}

function showMsg(scene, text) {
  msgText = scene.add.text(W/2, H/2, text, { fontSize: '32px', fill: '#fff', fontFamily: 'monospace' }).setOrigin(0.5);
}"##;

// ── Three.js 3D Scene ───────────────────────────────────────────────

const THREEJS_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{{NAME}}</title>
<style>
*{margin:0;padding:0}body{background:#000;overflow:hidden}canvas{display:block}
#ui{position:absolute;top:10px;left:10px;color:#aaa;font:13px/1.5 monospace;pointer-events:none}
</style>
</head>
<body>
<div id="ui">Mouse drag to orbit · Scroll to zoom</div>
<script type="importmap">
{ "imports": {
    "three": "https://cdn.jsdelivr.net/npm/three@0.170.0/build/three.module.js",
    "three/addons/": "https://cdn.jsdelivr.net/npm/three@0.170.0/examples/jsm/"
} }
</script>
<script type="module" src="game.js"></script>
</body>
</html>"##;

const THREEJS_JS: &str = r##"// {{NAME}} — Three.js 3D Scene
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';

// ── setup ──
const scene = new THREE.Scene();
scene.background = new THREE.Color(0x0a0a1a);
scene.fog = new THREE.Fog(0x0a0a1a, 25, 60);

const camera = new THREE.PerspectiveCamera(60, innerWidth / innerHeight, 0.1, 100);
camera.position.set(0, 8, 16);

const renderer = new THREE.WebGLRenderer({ antialias: true });
renderer.setSize(innerWidth, innerHeight);
renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
renderer.shadowMap.enabled = true;
document.body.appendChild(renderer.domElement);

const controls = new OrbitControls(camera, renderer.domElement);
controls.enableDamping = true;
controls.target.set(0, 2, 0);

// ── lights ──
scene.add(new THREE.AmbientLight(0x404060, 0.5));
const sun = new THREE.DirectionalLight(0xfff5e6, 1.2);
sun.position.set(10, 20, 10);
sun.castShadow = true;
sun.shadow.mapSize.set(2048, 2048);
scene.add(sun);

const glow = new THREE.PointLight(0x7b2cbf, 3, 18);
glow.position.set(0, 5, 0);
scene.add(glow);

// ── ground ──
const ground = new THREE.Mesh(
  new THREE.PlaneGeometry(50, 50),
  new THREE.MeshStandardMaterial({ color: 0x1e1e2e, roughness: 0.9 })
);
ground.rotation.x = -Math.PI / 2;
ground.receiveShadow = true;
scene.add(ground);
scene.add(new THREE.GridHelper(50, 50, 0x2a2a3a, 0x1e1e2e));

// ── central torusKnot ──
const torus = new THREE.Mesh(
  new THREE.TorusKnotGeometry(1.5, 0.45, 128, 24),
  new THREE.MeshStandardMaterial({ color: 0x7b2cbf, metalness: 0.8, roughness: 0.15 })
);
torus.position.y = 4.5;
torus.castShadow = true;
scene.add(torus);

// ── pillars ──
const palette = [0x2ec4b6, 0xff6b6b, 0xffd93d, 0x6c5ce7, 0x00b894];
for (let i = 0; i < 10; i++) {
  const a = (i / 10) * Math.PI * 2, r = 8;
  const h = 1.5 + Math.random() * 3.5;
  const box = new THREE.Mesh(
    new THREE.BoxGeometry(0.9, h, 0.9),
    new THREE.MeshStandardMaterial({ color: palette[i % 5], metalness: 0.3, roughness: 0.5 })
  );
  box.position.set(Math.cos(a) * r, h / 2, Math.sin(a) * r);
  box.castShadow = true;
  box.receiveShadow = true;
  scene.add(box);
}

// ── floating spheres ──
const spheres = [];
for (let i = 0; i < 16; i++) {
  const s = new THREE.Mesh(
    new THREE.SphereGeometry(0.25 + Math.random() * 0.2, 20, 20),
    new THREE.MeshStandardMaterial({
      color: palette[i % 5], emissive: palette[i % 5], emissiveIntensity: 0.35
    })
  );
  s.position.set((Math.random() - .5) * 18, 2 + Math.random() * 7, (Math.random() - .5) * 18);
  s.userData = { baseY: s.position.y, speed: .4 + Math.random(), off: Math.random() * 6.28 };
  scene.add(s);
  spheres.push(s);
}

// ── animate ──
const clock = new THREE.Clock();
(function tick() {
  requestAnimationFrame(tick);
  const t = clock.getElapsedTime();
  torus.rotation.x = t * 0.3;
  torus.rotation.y = t * 0.5;
  glow.intensity = 3 + Math.sin(t * 2) * 0.8;
  spheres.forEach(s => { s.position.y = s.userData.baseY + Math.sin(t * s.userData.speed + s.userData.off) * 0.6; });
  controls.update();
  renderer.render(scene, camera);
})();

addEventListener('resize', () => {
  camera.aspect = innerWidth / innerHeight;
  camera.updateProjectionMatrix();
  renderer.setSize(innerWidth, innerHeight);
});"##;

// ── Babylon.js 3D Game ──────────────────────────────────────────────

const BABYLON_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{{NAME}}</title>
<style>
*{margin:0;padding:0}body{overflow:hidden}
#renderCanvas{width:100vw;height:100vh;display:block;outline:none}
#ui{position:absolute;top:10px;left:10px;color:#fff;font:13px/1.6 monospace;pointer-events:none;text-shadow:0 1px 4px #000}
</style>
</head>
<body>
<div id="ui">WASD to move · Collect the orbs!</div>
<canvas id="renderCanvas"></canvas>
<script src="https://cdn.babylonjs.com/babylon.js"></script>
<script src="game.js"></script>
</body>
</html>"##;

const BABYLON_JS: &str = r##"// {{NAME}} — Babylon.js 3D Collector Game
// WASD to move, collect glowing orbs, dodge red obstacles

const canvas = document.getElementById('renderCanvas');
const engine = new BABYLON.Engine(canvas, true);
const ui = document.getElementById('ui');

function createScene() {
  const scene = new BABYLON.Scene(engine);
  scene.clearColor = new BABYLON.Color4(0.04, 0.04, 0.1, 1);
  scene.ambientColor = new BABYLON.Color3(0.1, 0.1, 0.15);

  // ── camera ──
  const cam = new BABYLON.ArcRotateCamera('cam', -Math.PI / 2, Math.PI / 3.5, 18, BABYLON.Vector3.Zero(), scene);
  cam.attachControl(canvas, true);
  cam.lowerRadiusLimit = 8;
  cam.upperRadiusLimit = 35;

  // ── lights ──
  new BABYLON.HemisphericLight('hemi', new BABYLON.Vector3(0, 1, 0), scene).intensity = 0.4;
  const sun = new BABYLON.DirectionalLight('sun', new BABYLON.Vector3(-1, -3, -1), scene);
  sun.intensity = 0.7;

  // ── ground ──
  const ground = BABYLON.MeshBuilder.CreateGround('ground', { width: 30, height: 30, subdivisions: 2 }, scene);
  const gmat = new BABYLON.StandardMaterial('gmat', scene);
  gmat.diffuseColor = new BABYLON.Color3(0.12, 0.12, 0.2);
  gmat.specularColor = BABYLON.Color3.Black();
  ground.material = gmat;

  // ── player ──
  const player = BABYLON.MeshBuilder.CreateSphere('player', { diameter: 1.2, segments: 20 }, scene);
  player.position.y = 0.6;
  const pmat = new BABYLON.StandardMaterial('pmat', scene);
  pmat.diffuseColor = new BABYLON.Color3(0.18, 0.77, 0.71);
  pmat.emissiveColor = new BABYLON.Color3(0.06, 0.25, 0.22);
  player.material = pmat;

  const playerGlow = new BABYLON.PointLight('pglow', player.position, scene);
  playerGlow.diffuse = new BABYLON.Color3(0.18, 0.77, 0.71);
  playerGlow.intensity = 0.6;
  playerGlow.range = 6;

  // ── collectible orbs ──
  let score = 0, total = 12;
  const orbs = [];
  for (let i = 0; i < total; i++) {
    const orb = BABYLON.MeshBuilder.CreateSphere('orb' + i, { diameter: 0.5, segments: 12 }, scene);
    const a = (i / total) * Math.PI * 2;
    const r = 4 + Math.random() * 8;
    orb.position = new BABYLON.Vector3(Math.cos(a) * r, 0.8 + Math.random() * 2, Math.sin(a) * r);
    const mat = new BABYLON.StandardMaterial('omat' + i, scene);
    mat.diffuseColor = new BABYLON.Color3(1, 0.85, 0.2);
    mat.emissiveColor = new BABYLON.Color3(0.4, 0.35, 0.05);
    orb.material = mat;
    orb._baseY = orb.position.y;
    orb._phase = Math.random() * 6.28;
    orbs.push(orb);
  }

  // ── obstacles ──
  const obstacles = [];
  for (let i = 0; i < 6; i++) {
    const obs = BABYLON.MeshBuilder.CreateBox('obs' + i, { size: 1.2 }, scene);
    const a = (i / 6) * Math.PI * 2 + 0.5;
    obs.position = new BABYLON.Vector3(Math.cos(a) * 7, 0.6, Math.sin(a) * 7);
    const mat = new BABYLON.StandardMaterial('obsmat' + i, scene);
    mat.diffuseColor = new BABYLON.Color3(1, 0.3, 0.3);
    mat.emissiveColor = new BABYLON.Color3(0.3, 0.05, 0.05);
    obs.material = mat;
    obs._center = obs.position.clone();
    obs._radius = 3 + Math.random() * 3;
    obs._speed = 0.5 + Math.random() * 1.5;
    obs._phase = Math.random() * 6.28;
    obstacles.push(obs);
  }

  // ── movement ──
  const speed = 0.12;
  const keys = {};
  window.addEventListener('keydown', e => keys[e.key.toLowerCase()] = true);
  window.addEventListener('keyup', e => keys[e.key.toLowerCase()] = false);

  let t = 0, alive = true;
  scene.onBeforeRenderObservable.add(() => {
    t += engine.getDeltaTime() / 1000;

    if (alive) {
      let dx = 0, dz = 0;
      if (keys['w'] || keys['arrowup']) dz = -speed;
      if (keys['s'] || keys['arrowdown']) dz = speed;
      if (keys['a'] || keys['arrowleft']) dx = -speed;
      if (keys['d'] || keys['arrowright']) dx = speed;
      player.position.x = Math.max(-14, Math.min(14, player.position.x + dx));
      player.position.z = Math.max(-14, Math.min(14, player.position.z + dz));
      playerGlow.position = player.position;
    }

    // orbit obstacles
    obstacles.forEach(o => {
      o.position.x = o._center.x + Math.cos(t * o._speed + o._phase) * o._radius;
      o.position.z = o._center.z + Math.sin(t * o._speed + o._phase) * o._radius;
      o.rotation.y += 0.02;
    });

    // float orbs
    orbs.forEach(o => {
      if (!o.isDisposed()) o.position.y = o._baseY + Math.sin(t * 2 + o._phase) * 0.3;
    });

    // collision: orbs
    orbs.forEach((o, i) => {
      if (o.isDisposed()) return;
      if (BABYLON.Vector3.Distance(player.position, o.position) < 1) {
        o.dispose();
        score++;
        ui.textContent = score >= total ? '🎉 You collected all orbs!' : `Orbs: ${score}/${total} · WASD to move`;
      }
    });

    // collision: obstacles
    if (alive) {
      for (const o of obstacles) {
        if (BABYLON.Vector3.Distance(player.position, o.position) < 1.1) {
          alive = false;
          pmat.emissiveColor = new BABYLON.Color3(0.4, 0.05, 0.05);
          pmat.diffuseColor = new BABYLON.Color3(0.6, 0.2, 0.2);
          ui.textContent = '💀 Hit! Press R to restart';
          break;
        }
      }
    }

    cam.target = BABYLON.Vector3.Lerp(cam.target, player.position, 0.08);
  });

  window.addEventListener('keydown', e => {
    if (e.key.toLowerCase() === 'r') location.reload();
  });

  return scene;
}

const scene = createScene();
engine.runRenderLoop(() => scene.render());
window.addEventListener('resize', () => engine.resize());"##;

// ── Godot 4 Project ─────────────────────────────────────────────────

const GODOT_PROJECT: &str = r##"; Engine configuration file.
; It's best edited using the editor UI and not directly,
; since the parameters that go here are not all obvious.
;
; Format:
;   [section] ; section goes between []
;   param=value ; assign values to parameters

config_version=5

[application]

config/name="{{NAME}}"
run/main_scene="res://scenes/main.tscn"
config/features=PackedStringArray("4.3", "Forward+")
config/icon="res://icon.svg"

[display]

window/size/viewport_width=1280
window/size/viewport_height=720
window/stretch/mode="canvas_items"

[input]

move_forward={
"deadzone": 0.5,
"events": [Object(InputEventKey,"resource_local_to_scene":false,"resource_name":"","device":-1,"window_id":0,"alt_pressed":false,"shift_pressed":false,"ctrl_pressed":false,"meta_pressed":false,"pressed":false,"keycode":0,"physical_keycode":87,"key_label":0,"unicode":119,"location":0,"echo":false,"script":null)
]
}
move_back={
"deadzone": 0.5,
"events": [Object(InputEventKey,"resource_local_to_scene":false,"resource_name":"","device":-1,"window_id":0,"alt_pressed":false,"shift_pressed":false,"ctrl_pressed":false,"meta_pressed":false,"pressed":false,"keycode":0,"physical_keycode":83,"key_label":0,"unicode":115,"location":0,"echo":false,"script":null)
]
}
move_left={
"deadzone": 0.5,
"events": [Object(InputEventKey,"resource_local_to_scene":false,"resource_name":"","device":-1,"window_id":0,"alt_pressed":false,"shift_pressed":false,"ctrl_pressed":false,"meta_pressed":false,"pressed":false,"keycode":0,"physical_keycode":65,"key_label":0,"unicode":97,"location":0,"echo":false,"script":null)
]
}
move_right={
"deadzone": 0.5,
"events": [Object(InputEventKey,"resource_local_to_scene":false,"resource_name":"","device":-1,"window_id":0,"alt_pressed":false,"shift_pressed":false,"ctrl_pressed":false,"meta_pressed":false,"pressed":false,"keycode":0,"physical_keycode":68,"key_label":0,"unicode":100,"location":0,"echo":false,"script":null)
]
}
jump={
"deadzone": 0.5,
"events": [Object(InputEventKey,"resource_local_to_scene":false,"resource_name":"","device":-1,"window_id":0,"alt_pressed":false,"shift_pressed":false,"ctrl_pressed":false,"meta_pressed":false,"pressed":false,"keycode":0,"physical_keycode":32,"key_label":0,"unicode":32,"location":0,"echo":false,"script":null)
]
}
shoot={
"deadzone": 0.5,
"events": [Object(InputEventMouseButton,"resource_local_to_scene":false,"resource_name":"","device":-1,"window_id":0,"alt_pressed":false,"shift_pressed":false,"ctrl_pressed":false,"meta_pressed":false,"button_mask":1,"position":Vector2(0,0),"global_position":Vector2(0,0),"factor":1.0,"button_index":1,"canceled":false,"pressed":true,"double_click":false,"script":null)
]
}

[rendering]

renderer/rendering_method="forward_plus"
anti_aliasing/quality/msaa_3d=2
"##;

const GODOT_MAIN_SCENE: &str = r##"[gd_scene load_steps=6 format=3 uid="uid://main_scene"]

[sub_resource type="ProceduralSkyMaterial" id="ProceduralSkyMaterial_sky"]
sky_top_color = Color(0.18, 0.22, 0.35, 1)
sky_horizon_color = Color(0.45, 0.55, 0.7, 1)
ground_bottom_color = Color(0.12, 0.1, 0.08, 1)
ground_horizon_color = Color(0.45, 0.55, 0.7, 1)

[sub_resource type="Sky" id="Sky_env"]
sky_material = SubResource("ProceduralSkyMaterial_sky")

[sub_resource type="Environment" id="Environment_world"]
background_mode = 2
sky = SubResource("Sky_env")
tonemap_mode = 2
ssao_enabled = true
glow_enabled = true
fog_enabled = true
fog_light_color = Color(0.6, 0.65, 0.75, 1)
fog_density = 0.002

[sub_resource type="StandardMaterial3D" id="StandardMaterial3D_ground"]
albedo_color = Color(0.3, 0.35, 0.25, 1)
roughness = 0.9

[sub_resource type="BoxShape3D" id="BoxShape3D_ground"]
size = Vector3(100, 1, 100)

[node name="Main" type="Node3D"]

[node name="WorldEnvironment" type="WorldEnvironment" parent="."]
environment = SubResource("Environment_world")

[node name="DirectionalLight3D" type="DirectionalLight3D" parent="."]
transform = Transform3D(1, 0, 0, 0, 0.707, 0.707, 0, -0.707, 0.707, 0, 20, 0)
shadow_enabled = true
directional_shadow_max_distance = 80.0

[node name="Ground" type="StaticBody3D" parent="."]
transform = Transform3D(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, -0.5, 0)

[node name="MeshInstance3D" type="MeshInstance3D" parent="Ground"]
mesh = ExtResource("") ; will use BoxMesh
material_override = SubResource("StandardMaterial3D_ground")

[node name="CollisionShape3D" type="CollisionShape3D" parent="Ground"]
shape = SubResource("BoxShape3D_ground")

[node name="Player" type="CharacterBody3D" parent="." groups=["player"]]
transform = Transform3D(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 1, 0)
script = ExtResource("") ; player.gd
"##;

const GODOT_PLAYER_GD: &str = r##"extends CharacterBody3D
## FPS Player Controller — WASD + mouse look + jump + shoot

const SPEED := 7.0
const SPRINT_SPEED := 12.0
const JUMP_VELOCITY := 6.0
const MOUSE_SENSITIVITY := 0.002
const GRAVITY := 20.0

@onready var camera: Camera3D = $Camera3D
@onready var ray: RayCast3D = $Camera3D/RayCast3D

var _sprinting := false

func _ready() -> void:
	Input.mouse_mode = Input.MOUSE_MODE_CAPTURED

func _unhandled_input(event: InputEvent) -> void:
	# Mouse look
	if event is InputEventMouseMotion and Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
		rotate_y(-event.relative.x * MOUSE_SENSITIVITY)
		camera.rotate_x(-event.relative.y * MOUSE_SENSITIVITY)
		camera.rotation.x = clampf(camera.rotation.x, -PI / 2.0, PI / 2.0)

	# Toggle mouse capture with Escape
	if event.is_action_pressed("ui_cancel"):
		if Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
			Input.mouse_mode = Input.MOUSE_MODE_VISIBLE
		else:
			Input.mouse_mode = Input.MOUSE_MODE_CAPTURED

	# Shoot
	if event.is_action_pressed("shoot") and Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
		_shoot()

func _physics_process(delta: float) -> void:
	# Gravity
	if not is_on_floor():
		velocity.y -= GRAVITY * delta

	# Jump
	if Input.is_action_just_pressed("jump") and is_on_floor():
		velocity.y = JUMP_VELOCITY

	# Sprint
	_sprinting = Input.is_key_pressed(KEY_SHIFT)

	# Movement direction
	var input_dir := Input.get_vector("move_left", "move_right", "move_forward", "move_back")
	var direction := (transform.basis * Vector3(input_dir.x, 0, input_dir.y)).normalized()
	var speed := SPRINT_SPEED if _sprinting else SPEED

	if direction:
		velocity.x = direction.x * speed
		velocity.z = direction.z * speed
	else:
		velocity.x = move_toward(velocity.x, 0, speed * 2.0 * delta)
		velocity.z = move_toward(velocity.z, 0, speed * 2.0 * delta)

	move_and_slide()

func _shoot() -> void:
	if ray and ray.is_colliding():
		var collider := ray.get_collider()
		if collider.has_method("take_damage"):
			collider.take_damage(25.0)
		elif collider is RigidBody3D:
			var dir := -ray.global_transform.basis.z
			collider.apply_impulse(dir * 10.0, ray.get_collision_point() - collider.global_position)
"##;

const GODOT_HUD_GD: &str = r##"extends CanvasLayer
## Minimal HUD — crosshair + FPS counter + health bar

@onready var fps_label: Label = $FPSLabel
@onready var crosshair: TextureRect = $Crosshair

func _ready() -> void:
	# Simple crosshair drawn as a centered + sign
	pass

func _process(_delta: float) -> void:
	if fps_label:
		fps_label.text = "FPS: %d" % Engine.get_frames_per_second()
"##;

const GODOT_ICON_SVG: &str = r##"<svg height="128" width="128" xmlns="http://www.w3.org/2000/svg"><rect fill="#363d52" width="128" height="128" rx="16"/><text x="50%" y="54%" dominant-baseline="middle" text-anchor="middle" fill="#8da0cb" font-family="monospace" font-size="48" font-weight="bold">G</text></svg>"##;

// ─────────────────────────────────────────────────────────────────────

async fn scaffold_web(dir: &Path, proj: &str, html: &str, js: &str) -> Result<String, String> {
    fs::write(dir.join("index.html"), tpl(html, proj))
        .await
        .map_err(|e| format!("写入 index.html 失败: {e}"))?;
    fs::write(dir.join("game.js"), tpl(js, proj))
        .await
        .map_err(|e| format!("写入 game.js 失败: {e}"))?;
    Ok(format!(
        "✅ 游戏项目「{proj}」已创建（网页引擎）\n\n\
         📁 {proj}/\n\
         ├── index.html  — 入口（CDN 加载引擎库）\n\
         └── game.js     — 游戏逻辑（修改这个文件）\n\n\
         ▸ 浏览器打开 index.html 即可运行\n\
         ▸ 修改 game.js 后刷新页面看效果\n\
         ▸ 完成后用 deploy_site 部署到 xxx.michaelide.xyz"
    ))
}

async fn scaffold_godot(dir: &Path, proj: &str) -> Result<String, String> {
    let scenes = dir.join("scenes");
    let scripts = dir.join("scripts");
    fs::create_dir_all(&scenes)
        .await
        .map_err(|e| format!("mkdir scenes: {e}"))?;
    fs::create_dir_all(&scripts)
        .await
        .map_err(|e| format!("mkdir scripts: {e}"))?;

    fs::write(dir.join("project.godot"), tpl(GODOT_PROJECT, proj))
        .await
        .map_err(|e| format!("写入 project.godot: {e}"))?;
    fs::write(scenes.join("main.tscn"), tpl(GODOT_MAIN_SCENE, proj))
        .await
        .map_err(|e| format!("写入 main.tscn: {e}"))?;
    fs::write(scripts.join("player.gd"), tpl(GODOT_PLAYER_GD, proj))
        .await
        .map_err(|e| format!("写入 player.gd: {e}"))?;
    fs::write(scripts.join("hud.gd"), tpl(GODOT_HUD_GD, proj))
        .await
        .map_err(|e| format!("写入 hud.gd: {e}"))?;
    fs::write(dir.join("icon.svg"), GODOT_ICON_SVG)
        .await
        .map_err(|e| format!("写入 icon.svg: {e}"))?;

    Ok(format!(
        "✅ Godot 4 项目「{proj}」已创建\n\n\
         📁 {proj}/\n\
         ├── project.godot   — 项目配置（引擎版本/输入映射/渲染设置）\n\
         ├── icon.svg        — 项目图标\n\
         ├── scenes/\n\
         │   └── main.tscn   — 主场景（环境/灯光/地面/玩家）\n\
         └── scripts/\n\
             ├── player.gd   — FPS 玩家控制器（WASD+鼠标+跳跃+射击）\n\
             └── hud.gd      — HUD（准星/FPS计数器）\n\n\
         ▸ 用 Godot 4 编辑器打开此文件夹即可运行\n\
         ▸ 没装 Godot？→ brew install godot 或去 godotengine.org 下载\n\
         ▸ 在 IDE 里编辑 .gd 脚本，Godot 编辑器实时热重载\n\
         ▸ 导出：项目 → 导出 → 选平台（Windows/macOS/Linux/Android/iOS/Web）\n\n\
         💡 Godot 4 能做 3A 级 3D 游戏：Vulkan 渲染 / GI / SSAO / 物理 / 动画树 / 全平台导出"
    ))
}

#[tauri::command]
pub async fn game_scaffold(
    engine: String,
    name: String,
    workspace: String,
) -> Result<String, String> {
    let eng = engine.trim().to_lowercase();
    let proj = if name.trim().is_empty() {
        "my-game".to_string()
    } else {
        name.trim()
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>()
    };

    let dir = PathBuf::from(&workspace).join(&proj);
    fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("创建目录失败: {e}"))?;

    match eng.as_str() {
        "godot" | "godot4" | "gd" | "gdscript" => scaffold_godot(&dir, &proj).await,
        "threejs" | "three.js" | "three" => {
            scaffold_web(&dir, &proj, THREEJS_HTML, THREEJS_JS).await
        }
        "babylon" | "babylonjs" | "babylon.js" => {
            scaffold_web(&dir, &proj, BABYLON_HTML, BABYLON_JS).await
        }
        "phaser" | "2d" => scaffold_web(&dir, &proj, PHASER_HTML, PHASER_JS).await,
        _ => {
            // Default: 3D games → Godot, 2D → Phaser
            scaffold_godot(&dir, &proj).await
        }
    }
}
