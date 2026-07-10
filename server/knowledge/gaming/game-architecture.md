# Gaming: Game Architecture & Engine Patterns

## Game Loop Fundamentals

### Fixed Timestep Loop
```javascript
const TICK_RATE = 60;
const TICK_DURATION = 1000 / TICK_RATE;  // 16.67ms

let previousTime = performance.now();
let accumulator = 0;

function gameLoop(currentTime) {
  const deltaTime = currentTime - previousTime;
  previousTime = currentTime;
  accumulator += deltaTime;

  // Fixed-step physics/logic (deterministic)
  while (accumulator >= TICK_DURATION) {
    update(TICK_DURATION / 1000);  // always same dt
    accumulator -= TICK_DURATION;
  }

  // Variable-step rendering (interpolated)
  const alpha = accumulator / TICK_DURATION;
  render(alpha);  // interpolate between previous and current state

  requestAnimationFrame(gameLoop);
}
```

### Entity Component System (ECS)
```typescript
// Components are pure data — no methods
interface Position { x: number; y: number; }
interface Velocity { dx: number; dy: number; }
interface Sprite { texture: string; width: number; height: number; }
interface Health { current: number; max: number; }
interface Collider { radius: number; }

// Systems operate on component queries
function movementSystem(world, dt) {
  for (const [entity, pos, vel] of world.query(Position, Velocity)) {
    pos.x += vel.dx * dt;
    pos.y += vel.dy * dt;
  }
}

function collisionSystem(world) {
  const collidables = world.query(Position, Collider);
  for (let i = 0; i < collidables.length; i++) {
    for (let j = i + 1; j < collidables.length; j++) {
      const [, posA, colA] = collidables[i];
      const [, posB, colB] = collidables[j];
      const dist = Math.hypot(posA.x - posB.x, posA.y - posB.y);
      if (dist < colA.radius + colB.radius) {
        world.emit('collision', { a: collidables[i][0], b: collidables[j][0] });
      }
    }
  }
}
```

## Engine-Specific Patterns

### Unity (C#)
```csharp
// MonoBehaviour lifecycle: Awake → OnEnable → Start → Update → LateUpdate → OnDisable → OnDestroy
public class Enemy : MonoBehaviour
{
    [SerializeField] private float speed = 5f;
    [SerializeField] private int maxHealth = 100;
    private int _health;

    private void Awake() { _health = maxHealth; }

    private void Update()
    {
        transform.Translate(Vector3.forward * speed * Time.deltaTime);
    }

    public void TakeDamage(int amount)
    {
        _health -= amount;
        if (_health <= 0) Die();
    }

    private void Die()
    {
        // Return to pool instead of Destroy (performance)
        ObjectPool.Instance.Return(gameObject);
    }
}

// Object Pooling (avoid GC spikes)
public class ObjectPool : MonoBehaviour
{
    private Queue<GameObject> _pool = new();
    [SerializeField] private GameObject prefab;
    [SerializeField] private int initialSize = 20;

    private void Awake()
    {
        for (int i = 0; i < initialSize; i++)
        {
            var obj = Instantiate(prefab);
            obj.SetActive(false);
            _pool.Enqueue(obj);
        }
    }

    public GameObject Get()
    {
        var obj = _pool.Count > 0 ? _pool.Dequeue() : Instantiate(prefab);
        obj.SetActive(true);
        return obj;
    }

    public void Return(GameObject obj)
    {
        obj.SetActive(false);
        _pool.Enqueue(obj);
    }
}
```

### Unreal Engine 5 (C++)
```cpp
// Actor lifecycle: Constructor → BeginPlay → Tick → EndPlay
UCLASS()
class AWeapon : public AActor
{
    GENERATED_BODY()

public:
    AWeapon();

protected:
    virtual void BeginPlay() override;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Weapon")
    float Damage = 25.0f;

    UPROPERTY(EditAnywhere, Category = "Weapon")
    float FireRate = 0.5f;

    UFUNCTION(BlueprintCallable, Category = "Weapon")
    void Fire();

    UPROPERTY(VisibleAnywhere)
    UStaticMeshComponent* MeshComp;
};

// Common UE5 LLM mistakes:
// 1. Missing GENERATED_BODY() macro
// 2. Using raw pointers instead of TObjectPtr<> (UE5.1+)
// 3. Forgetting Super::BeginPlay() call
// 4. NewObject<> for Actors (use SpawnActor<> instead)
// 5. Not marking UPROPERTY() on UObject pointers (GC will collect them)
```

### Godot 4 (GDScript)
```gdscript
# Node lifecycle: _init → _ready → _process/_physics_process → _exit_tree
extends CharacterBody2D

@export var speed: float = 200.0
@export var jump_velocity: float = -400.0

var gravity: float = ProjectSettings.get_setting("physics/2d/default_gravity")

func _physics_process(delta: float) -> void:
    if not is_on_floor():
        velocity.y += gravity * delta

    if Input.is_action_just_pressed("jump") and is_on_floor():
        velocity.y = jump_velocity

    var direction := Input.get_axis("move_left", "move_right")
    velocity.x = direction * speed if direction else move_toward(velocity.x, 0, speed)

    move_and_slide()

# Signals (Godot's event system)
signal health_changed(new_health: int)
signal died

func take_damage(amount: int) -> void:
    health -= amount
    health_changed.emit(health)
    if health <= 0:
        died.emit()
```

## Finite State Machine (Game AI)

```python
class StateMachine:
    def __init__(self, initial_state):
        self.states = {}
        self.current = initial_state

    def add_state(self, name, on_enter=None, on_update=None, on_exit=None, transitions=None):
        self.states[name] = {
            'enter': on_enter or (lambda: None),
            'update': on_update or (lambda dt: None),
            'exit': on_exit or (lambda: None),
            'transitions': transitions or {},
        }

    def transition(self, event):
        state = self.states[self.current]
        if event in state['transitions']:
            state['exit']()
            self.current = state['transitions'][event]
            self.states[self.current]['enter']()

    def update(self, dt):
        self.states[self.current]['update'](dt)

# Enemy AI example
# States: idle → patrol → chase → attack → flee
# Transitions: see_player → chase, in_range → attack, health_low → flee, lost_player → patrol
```

## Multiplayer Networking

### Client-Server Architecture
```
Server (authoritative):
  - Runs game simulation at fixed tick rate (20-64 Hz)
  - Validates all player inputs
  - Broadcasts state to clients

Client:
  - Sends inputs to server (not positions)
  - Predicts locally (client-side prediction)
  - Reconciles with server state (server correction)
  - Interpolates other entities (entity interpolation)
```

### Netcode Patterns
```javascript
// Client-side prediction + server reconciliation
class NetworkedPlayer {
  constructor() {
    this.pendingInputs = [];  // inputs waiting for server ack
    this.inputSequence = 0;
  }

  sendInput(input) {
    input.sequence = this.inputSequence++;
    this.pendingInputs.push(input);
    this.applyInput(input);  // predict locally
    socket.send({ type: 'input', ...input });
  }

  onServerState(state) {
    // Remove acknowledged inputs
    this.pendingInputs = this.pendingInputs.filter(i => i.sequence > state.lastProcessedInput);

    // Reset to server state
    this.position = state.position;

    // Re-apply pending (unacknowledged) inputs
    for (const input of this.pendingInputs) {
      this.applyInput(input);
    }
  }
}

// Entity interpolation (other players)
class InterpolatedEntity {
  constructor() {
    this.buffer = [];  // [(timestamp, state), ...]
    this.interpolationDelay = 100;  // ms behind server
  }

  addState(timestamp, state) {
    this.buffer.push({ timestamp, state });
    if (this.buffer.length > 10) this.buffer.shift();
  }

  getInterpolatedState(renderTime) {
    const targetTime = renderTime - this.interpolationDelay;

    for (let i = 0; i < this.buffer.length - 1; i++) {
      const a = this.buffer[i], b = this.buffer[i + 1];
      if (a.timestamp <= targetTime && targetTime <= b.timestamp) {
        const t = (targetTime - a.timestamp) / (b.timestamp - a.timestamp);
        return lerp(a.state, b.state, t);
      }
    }
    return this.buffer[this.buffer.length - 1]?.state;
  }
}
```

## Performance Optimization

### Common Patterns
```
1. Object pooling: pre-allocate, reuse, never Destroy/new in hot path
2. Spatial partitioning: quadtree (2D), octree (3D), grid for broad-phase collision
3. LOD (Level of Detail): reduce mesh/texture quality at distance
4. Culling: frustum culling (engine handles), occlusion culling (manual for complex scenes)
5. Batching: combine draw calls (static batching, GPU instancing)
6. Fixed-point math: for deterministic netcode (avoid float desync across platforms)
7. Data-oriented design: struct-of-arrays over array-of-structs (cache locality)
```

### Memory Budget (Mobile)
```
Target 60 FPS → 16.67ms per frame budget:
- Input:     0.5ms
- Physics:   2-3ms
- AI:        1-2ms
- Animation: 1-2ms
- Rendering: 8-10ms
- Audio:     0.5ms
- Headroom:  2ms (for GC spikes, OS interrupts)

Memory targets (mobile):
- Total RAM: < 500MB (iOS kills at ~1.5GB on most devices)
- Textures: < 200MB (use compressed formats: ASTC, ETC2)
- Audio: < 50MB (use streaming for music, loaded for SFX)
- Meshes: < 100MB
```

## Web Games (HTML5 / WebGL)

### Three.js Scene Setup
```javascript
import * as THREE from 'three';

const scene = new THREE.Scene();
const camera = new THREE.PerspectiveCamera(75, window.innerWidth / window.innerHeight, 0.1, 1000);
const renderer = new THREE.WebGLRenderer({ antialias: true });
renderer.setSize(window.innerWidth, window.innerHeight);
renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));  // cap for performance

function animate() {
  requestAnimationFrame(animate);
  // update logic
  renderer.render(scene, camera);
}
animate();

// Cleanup on unmount (critical in React/SPA)
function dispose() {
  renderer.dispose();
  scene.traverse(obj => {
    if (obj.geometry) obj.geometry.dispose();
    if (obj.material) {
      if (Array.isArray(obj.material)) obj.material.forEach(m => m.dispose());
      else obj.material.dispose();
    }
  });
}
```

### 2D Canvas Game Pattern
```javascript
const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');

const entities = [];
let lastTime = 0;

function loop(timestamp) {
  const dt = (timestamp - lastTime) / 1000;
  lastTime = timestamp;

  // Update
  for (const entity of entities) entity.update(dt);

  // Collision (simple AABB)
  for (let i = 0; i < entities.length; i++) {
    for (let j = i + 1; j < entities.length; j++) {
      if (aabb(entities[i], entities[j])) {
        entities[i].onCollision(entities[j]);
      }
    }
  }

  // Render
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  for (const entity of entities) entity.render(ctx);

  requestAnimationFrame(loop);
}
requestAnimationFrame(loop);
```
