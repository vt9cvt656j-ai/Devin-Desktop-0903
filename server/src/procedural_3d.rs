use serde_json::json;

pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

pub struct SceneNode {
    pub mesh: Mesh,
    pub translation: [f32; 3],
    pub rotation_deg: [f32; 3],
    pub scale: [f32; 3],
    pub color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
}

// ── Primitives ───────────────────────────────────────────────

pub fn cube(size: f32) -> Mesh {
    let s = size / 2.0;
    let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
        (
            [0., 0., 1.],
            [[-s, -s, s], [s, -s, s], [s, s, s], [-s, s, s]],
        ),
        (
            [0., 0., -1.],
            [[s, -s, -s], [-s, -s, -s], [-s, s, -s], [s, s, -s]],
        ),
        (
            [0., 1., 0.],
            [[-s, s, s], [s, s, s], [s, s, -s], [-s, s, -s]],
        ),
        (
            [0., -1., 0.],
            [[-s, -s, -s], [s, -s, -s], [s, -s, s], [-s, -s, s]],
        ),
        (
            [1., 0., 0.],
            [[s, -s, s], [s, -s, -s], [s, s, -s], [s, s, s]],
        ),
        (
            [-1., 0., 0.],
            [[-s, -s, -s], [-s, -s, s], [-s, s, s], [-s, s, -s]],
        ),
    ];
    let mut p = Vec::with_capacity(24);
    let mut n = Vec::with_capacity(24);
    let mut idx = Vec::with_capacity(36);
    for (i, (norm, corners)) in faces.iter().enumerate() {
        let b = (i * 4) as u32;
        for c in corners {
            p.push(*c);
            n.push(*norm);
        }
        idx.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
    }
    Mesh {
        positions: p,
        normals: n,
        indices: idx,
    }
}

pub fn sphere(radius: f32, seg: u32) -> Mesh {
    let seg = seg.max(4);
    let rings = seg;
    let sectors = seg * 2;
    let mut p = Vec::new();
    let mut n = Vec::new();
    let mut idx = Vec::new();
    let pi = std::f32::consts::PI;
    for i in 0..=rings {
        let phi = pi * i as f32 / rings as f32;
        for j in 0..=sectors {
            let theta = 2.0 * pi * j as f32 / sectors as f32;
            let (x, y, z) = (phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
            p.push([x * radius, y * radius, z * radius]);
            n.push([x, y, z]);
        }
    }
    for i in 0..rings {
        for j in 0..sectors {
            let a = i * (sectors + 1) + j;
            let b = a + sectors + 1;
            idx.extend_from_slice(&[a, b, a + 1, b, b + 1, a + 1]);
        }
    }
    Mesh {
        positions: p,
        normals: n,
        indices: idx,
    }
}

pub fn cylinder(radius: f32, height: f32, seg: u32) -> Mesh {
    let seg = seg.max(3);
    let hh = height / 2.0;
    let pi2 = 2.0 * std::f32::consts::PI;
    let mut p = Vec::new();
    let mut n = Vec::new();
    let mut idx = Vec::new();
    for i in 0..=seg {
        let a = pi2 * i as f32 / seg as f32;
        let (x, z) = (a.cos(), a.sin());
        p.push([x * radius, -hh, z * radius]);
        n.push([x, 0., z]);
        p.push([x * radius, hh, z * radius]);
        n.push([x, 0., z]);
    }
    for i in 0..seg {
        let b = i * 2;
        idx.extend_from_slice(&[b, b + 2, b + 1, b + 1, b + 2, b + 3]);
    }
    for (y, ny) in [(hh, 1.0f32), (-hh, -1.0f32)] {
        let center = p.len() as u32;
        p.push([0., y, 0.]);
        n.push([0., ny, 0.]);
        for i in 0..=seg {
            let a = pi2 * i as f32 / seg as f32;
            p.push([a.cos() * radius, y, a.sin() * radius]);
            n.push([0., ny, 0.]);
        }
        for i in 0..seg {
            if ny > 0.0 {
                idx.extend_from_slice(&[center, center + 1 + i, center + 2 + i]);
            } else {
                idx.extend_from_slice(&[center, center + 2 + i, center + 1 + i]);
            }
        }
    }
    Mesh {
        positions: p,
        normals: n,
        indices: idx,
    }
}

pub fn cone(radius: f32, height: f32, seg: u32) -> Mesh {
    let seg = seg.max(3);
    let hh = height / 2.0;
    let pi2 = 2.0 * std::f32::consts::PI;
    let slope = radius / height;
    let mut p = Vec::new();
    let mut n = Vec::new();
    let mut idx = Vec::new();
    let apex = 0u32;
    p.push([0., hh, 0.]);
    n.push([0., 1., 0.]);
    for i in 0..=seg {
        let a = pi2 * i as f32 / seg as f32;
        let (x, z) = (a.cos(), a.sin());
        p.push([x * radius, -hh, z * radius]);
        let len = (x * x + slope * slope + z * z).sqrt();
        n.push([x / len, slope / len, z / len]);
    }
    for i in 0..seg {
        idx.extend_from_slice(&[apex, 1 + i, 2 + i]);
    }
    let bc = p.len() as u32;
    p.push([0., -hh, 0.]);
    n.push([0., -1., 0.]);
    for i in 0..=seg {
        let a = pi2 * i as f32 / seg as f32;
        p.push([a.cos() * radius, -hh, a.sin() * radius]);
        n.push([0., -1., 0.]);
    }
    for i in 0..seg {
        idx.extend_from_slice(&[bc, bc + 2 + i, bc + 1 + i]);
    }
    Mesh {
        positions: p,
        normals: n,
        indices: idx,
    }
}

pub fn torus(major: f32, minor: f32, seg: u32, tube_seg: u32) -> Mesh {
    let seg = seg.max(3);
    let ts = tube_seg.max(3);
    let pi2 = 2.0 * std::f32::consts::PI;
    let mut p = Vec::new();
    let mut n = Vec::new();
    let mut idx = Vec::new();
    for i in 0..=seg {
        let u = pi2 * i as f32 / seg as f32;
        let (cu, su) = (u.cos(), u.sin());
        for j in 0..=ts {
            let v = pi2 * j as f32 / ts as f32;
            let (cv, sv) = (v.cos(), v.sin());
            let x = (major + minor * cv) * cu;
            let y = minor * sv;
            let z = (major + minor * cv) * su;
            p.push([x, y, z]);
            n.push([cv * cu, sv, cv * su]);
        }
    }
    for i in 0..seg {
        for j in 0..ts {
            let a = i * (ts + 1) + j;
            let b = a + ts + 1;
            idx.extend_from_slice(&[a, b, a + 1, b, b + 1, a + 1]);
        }
    }
    Mesh {
        positions: p,
        normals: n,
        indices: idx,
    }
}

pub fn plane(w: f32, h: f32) -> Mesh {
    let (hw, hh) = (w / 2., h / 2.);
    Mesh {
        positions: vec![[-hw, 0., -hh], [hw, 0., -hh], [hw, 0., hh], [-hw, 0., hh]],
        normals: vec![[0., 1., 0.]; 4],
        indices: vec![0, 1, 2, 0, 2, 3],
    }
}

// ── GLB builder ──────────────────────────────────────────────

fn euler_to_quat(xd: f32, yd: f32, zd: f32) -> [f32; 4] {
    let (hx, hy, hz) = (
        xd.to_radians() / 2.,
        yd.to_radians() / 2.,
        zd.to_radians() / 2.,
    );
    let (sx, cx) = hx.sin_cos();
    let (sy, cy) = hy.sin_cos();
    let (sz, cz) = hz.sin_cos();
    [
        sx * cy * cz - cx * sy * sz,
        cx * sy * cz + sx * cy * sz,
        cx * cy * sz - sx * sy * cz,
        cx * cy * cz + sx * sy * sz,
    ]
}

fn bounds(positions: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut mn = [f32::MAX; 3];
    let mut mx = [f32::MIN; 3];
    for p in positions {
        for i in 0..3 {
            mn[i] = mn[i].min(p[i]);
            mx[i] = mx[i].max(p[i]);
        }
    }
    (mn, mx)
}

pub fn build_glb(nodes: &[SceneNode]) -> Vec<u8> {
    let mut bin: Vec<u8> = Vec::new();
    let mut bv = Vec::new(); // bufferViews
    let mut acc = Vec::new(); // accessors
    let mut meshes = Vec::new();
    let mut gn = Vec::new(); // gltf nodes
    let mut mats = Vec::new(); // materials
    let mut ni: Vec<serde_json::Value> = Vec::new();

    for (i, node) in nodes.iter().enumerate() {
        mats.push(json!({
            "pbrMetallicRoughness": {
                "baseColorFactor": node.color,
                "metallicFactor": node.metallic,
                "roughnessFactor": node.roughness
            }
        }));

        // positions
        let pb = bv.len();
        let po = bin.len();
        for v in &node.mesh.positions {
            for f in v {
                bin.extend_from_slice(&f.to_le_bytes());
            }
        }
        bv.push(json!({"buffer":0,"byteOffset":po,"byteLength":bin.len()-po,"target":34962}));
        let (mn, mx) = bounds(&node.mesh.positions);
        let pa = acc.len();
        acc.push(json!({"bufferView":pb,"componentType":5126,"count":node.mesh.positions.len(),"type":"VEC3","min":mn,"max":mx}));

        // normals
        let nb = bv.len();
        let no = bin.len();
        for v in &node.mesh.normals {
            for f in v {
                bin.extend_from_slice(&f.to_le_bytes());
            }
        }
        bv.push(json!({"buffer":0,"byteOffset":no,"byteLength":bin.len()-no,"target":34962}));
        let na = acc.len();
        acc.push(json!({"bufferView":nb,"componentType":5126,"count":node.mesh.normals.len(),"type":"VEC3"}));

        // indices
        let ib = bv.len();
        let io = bin.len();
        for idx in &node.mesh.indices {
            bin.extend_from_slice(&idx.to_le_bytes());
        }
        bv.push(json!({"buffer":0,"byteOffset":io,"byteLength":bin.len()-io,"target":34963}));
        let ia = acc.len();
        acc.push(json!({"bufferView":ib,"componentType":5125,"count":node.mesh.indices.len(),"type":"SCALAR"}));

        meshes.push(json!({"primitives":[{"attributes":{"POSITION":pa,"NORMAL":na},"indices":ia,"material":i}]}));
        let q = euler_to_quat(
            node.rotation_deg[0],
            node.rotation_deg[1],
            node.rotation_deg[2],
        );
        gn.push(json!({"mesh":i,"translation":node.translation,"rotation":q,"scale":node.scale}));
        ni.push(json!(i));
    }

    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }

    let gltf = json!({
        "asset": {"version":"2.0","generator":"MichaelIDE-Procedural3D"},
        "scene": 0,
        "scenes": [{"nodes": ni}],
        "nodes": gn,
        "meshes": meshes,
        "accessors": acc,
        "bufferViews": bv,
        "buffers": [{"byteLength": bin.len()}],
        "materials": mats
    });

    let mut jb = serde_json::to_string(&gltf).unwrap().into_bytes();
    while !jb.len().is_multiple_of(4) {
        jb.push(0x20);
    }

    let total = 12 + 8 + jb.len() + 8 + bin.len();
    let mut glb = Vec::with_capacity(total);
    glb.extend_from_slice(&0x46546C67u32.to_le_bytes()); // "glTF"
    glb.extend_from_slice(&2u32.to_le_bytes());
    glb.extend_from_slice(&(total as u32).to_le_bytes());
    glb.extend_from_slice(&(jb.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x4E4F534Au32.to_le_bytes()); // JSON
    glb.extend_from_slice(&jb);
    glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x004E4942u32.to_le_bytes()); // BIN
    glb.extend_from_slice(&bin);
    glb
}

// ── Scene JSON → SceneNode list ──────────────────────────────

fn get_f32(v: &serde_json::Value, key: &str, def: f32) -> f32 {
    v.get(key)
        .and_then(|x| x.as_f64())
        .map(|x| x as f32)
        // NaN / inf 会原样流进顶点缓冲，再进 GLB。谁读这个模型谁崩。非有限值一律退默认。
        .filter(|x| x.is_finite())
        .unwrap_or(def)
}

/// 单个基元的最大细分段数。
///
/// 这一条是入口处的硬闸，不是美学取舍。`segments` 之前从请求直接 `as u32`，只有下限、
/// 没有上限：sphere 的顶点数是 (seg+1)·(2·seg+1)，seg=20000 就是约 8 亿个顶点，同步
/// 分配、直接 OOM 掉整个网关进程 —— 聊天、计费、Stripe webhook 全部随之崩掉，而且一分钱
/// 余额的账号就能发这个请求。64 段已经足够圆滑，再高肉眼也分不出。
const MAX_SEGMENTS: u32 = 64;
/// 一个场景最多几个节点。50MB 的请求体能塞进几百万个平凡节点，每个都要分配几何。
const MAX_NODES: usize = 256;
/// 读段数并夹紧。NaN/inf 已在 get_f32 里挡掉，这里再兜一次下限。
fn seg_of(params: &serde_json::Value, floor: u32) -> u32 {
    (get_f32(params, "segments", 16.0) as u32).clamp(floor, MAX_SEGMENTS)
}

fn get_arr3(v: &serde_json::Value, key: &str, def: [f32; 3]) -> [f32; 3] {
    v.get(key)
        .and_then(|a| a.as_array())
        .map(|a| {
            [
                a.first().and_then(|x| x.as_f64()).unwrap_or(def[0] as f64) as f32,
                a.get(1).and_then(|x| x.as_f64()).unwrap_or(def[1] as f64) as f32,
                a.get(2).and_then(|x| x.as_f64()).unwrap_or(def[2] as f64) as f32,
            ]
        })
        .unwrap_or(def)
}

fn get_arr4(v: &serde_json::Value, key: &str, def: [f32; 4]) -> [f32; 4] {
    v.get(key)
        .and_then(|a| a.as_array())
        .map(|a| {
            [
                a.first().and_then(|x| x.as_f64()).unwrap_or(def[0] as f64) as f32,
                a.get(1).and_then(|x| x.as_f64()).unwrap_or(def[1] as f64) as f32,
                a.get(2).and_then(|x| x.as_f64()).unwrap_or(def[2] as f64) as f32,
                a.get(3).and_then(|x| x.as_f64()).unwrap_or(def[3] as f64) as f32,
            ]
        })
        .unwrap_or(def)
}

pub fn parse_scene(scene: &serde_json::Value) -> Vec<SceneNode> {
    let empty = vec![];
    let nodes = scene
        .get("nodes")
        .and_then(|n| n.as_array())
        .unwrap_or(&empty);
    nodes
        .iter()
        // 节点数封顶。50MB 请求体能装几百万个平凡节点，每个都要建几何、进 GLB。
        .take(MAX_NODES)
        .map(|n| {
            let shape = n.get("shape").and_then(|s| s.as_str()).unwrap_or("cube");
            let params = n.get("params").cloned().unwrap_or(json!({}));
            let mesh = match shape {
                "cube" => cube(get_f32(&params, "size", 1.0)),
                "sphere" => sphere(get_f32(&params, "radius", 0.5), seg_of(&params, 4)),
                "cylinder" => cylinder(
                    get_f32(&params, "radius", 0.5),
                    get_f32(&params, "height", 1.0),
                    seg_of(&params, 3),
                ),
                "cone" => cone(
                    get_f32(&params, "radius", 0.5),
                    get_f32(&params, "height", 1.0),
                    seg_of(&params, 3),
                ),
                "torus" => torus(
                    get_f32(&params, "major_radius", 0.5),
                    get_f32(&params, "minor_radius", 0.15),
                    seg_of(&params, 3),
                    seg_of(&params, 3),
                ),
                "plane" => plane(
                    get_f32(&params, "width", 1.0),
                    get_f32(&params, "height", 1.0),
                ),
                _ => cube(1.0),
            };
            SceneNode {
                mesh,
                translation: get_arr3(n, "position", [0., 0., 0.]),
                rotation_deg: get_arr3(n, "rotation", [0., 0., 0.]),
                scale: get_arr3(n, "scale", [1., 1., 1.]),
                color: get_arr4(n, "color", [0.8, 0.8, 0.8, 1.0]),
                metallic: get_f32(n, "metallic", 0.0),
                roughness: get_f32(n, "roughness", 0.5),
            }
        })
        .collect()
}

pub const SCENE_SYSTEM_PROMPT: &str = r#"You are a 3D scene composer for game assets. Convert descriptions into JSON scene graphs using primitives.

Available shapes: cube, sphere, cylinder, cone, torus, plane
Each node: {"shape":"...", "params":{shape-specific}, "position":[x,y,z], "rotation":[rx,ry,rz] degrees, "scale":[sx,sy,sz], "color":[r,g,b,a] 0-1, "metallic":0-1, "roughness":0-1}

Shape params:
- cube: {"size": 1.0}
- sphere: {"radius": 0.5, "segments": 16}
- cylinder: {"radius": 0.5, "height": 1.0, "segments": 16}
- cone: {"radius": 0.5, "height": 1.0, "segments": 16}
- torus: {"major_radius": 0.5, "minor_radius": 0.15, "segments": 16}
- plane: {"width": 1.0, "height": 1.0}

Example - a simple sword:
{"nodes":[
  {"shape":"cylinder","params":{"radius":0.02,"height":0.8},"position":[0,0.4,0],"color":[0.7,0.7,0.75,1],"metallic":0.9,"roughness":0.2},
  {"shape":"cube","params":{"size":0.12},"position":[0,0.02,0],"scale":[2.5,0.3,0.6],"color":[0.4,0.25,0.1,1],"metallic":0.1,"roughness":0.8},
  {"shape":"sphere","params":{"radius":0.04},"position":[0,-0.02,0],"color":[0.8,0.1,0.1,1],"metallic":0.3,"roughness":0.4},
  {"shape":"cube","params":{"size":0.06},"position":[0,0.84,0],"scale":[0.3,2.0,0.3],"color":[0.7,0.7,0.75,1],"metallic":0.9,"roughness":0.2}
]}

Example - a tree:
{"nodes":[
  {"shape":"cylinder","params":{"radius":0.15,"height":1.2},"position":[0,0.6,0],"color":[0.4,0.25,0.1,1],"roughness":0.9},
  {"shape":"sphere","params":{"radius":0.8,"segments":12},"position":[0,1.5,0],"color":[0.15,0.5,0.1,1],"roughness":0.8},
  {"shape":"sphere","params":{"radius":0.5,"segments":10},"position":[0.4,1.8,0.3],"color":[0.1,0.45,0.08,1],"roughness":0.85},
  {"shape":"sphere","params":{"radius":0.45,"segments":10},"position":[-0.3,1.9,-0.2],"color":[0.12,0.48,0.09,1],"roughness":0.82}
]}

Example - a table:
{"nodes":[
  {"shape":"cube","params":{"size":1.0},"position":[0,0.75,0],"scale":[1.5,0.05,0.8],"color":[0.55,0.35,0.15,1],"roughness":0.7},
  {"shape":"cylinder","params":{"radius":0.03,"height":0.7},"position":[-0.65,0.35,-0.32],"color":[0.45,0.28,0.12,1],"roughness":0.8},
  {"shape":"cylinder","params":{"radius":0.03,"height":0.7},"position":[0.65,0.35,-0.32],"color":[0.45,0.28,0.12,1],"roughness":0.8},
  {"shape":"cylinder","params":{"radius":0.03,"height":0.7},"position":[-0.65,0.35,0.32],"color":[0.45,0.28,0.12,1],"roughness":0.8},
  {"shape":"cylinder","params":{"radius":0.03,"height":0.7},"position":[0.65,0.35,0.32],"color":[0.45,0.28,0.12,1],"roughness":0.8}
]}

Rules:
- Use 5-30 nodes for good detail. Simple objects ~5, complex ~20-30.
- Pick realistic colors and materials (metallic for metal, rough for wood/stone).
- Position objects so they compose naturally (e.g. legs under tabletop).
- Output ONLY the JSON object {"nodes":[...]}, nothing else.
- Be creative: use scale to reshape primitives (flat cube = board, tall thin cylinder = pole)."#;

#[cfg(test)]
mod dos_guard_tests {
    use super::*;

    /// 一个请求打崩整个网关的那条路：segments 从请求直接读、无上限、同步分配。
    /// 这个测试守住入口处的三道闸：段数夹紧、节点封顶、NaN/inf 不入缓冲。
    #[test]
    fn a_hostile_scene_cannot_allocate_an_unbounded_mesh() {
        // 段数被夹到 MAX_SEGMENTS，无论请求写多大。
        let scene = json!({ "nodes": [
            { "shape": "sphere", "params": { "segments": 20000, "radius": 0.5 } }
        ]});
        let nodes = parse_scene(&scene);
        assert_eq!(nodes.len(), 1);
        // sphere(seg) 的顶点数 = (seg+1)*(2*seg+1)。seg 被夹到 64，顶点数应在万级，
        // 而不是 8 亿。用顶点数反推，别依赖内部字段名。
        let verts = nodes[0].mesh.positions.len();
        let cap = ((MAX_SEGMENTS + 1) * (2 * MAX_SEGMENTS + 1)) as usize;
        assert!(verts <= cap, "顶点数 {verts} 超过 seg={MAX_SEGMENTS} 的上限 {cap}");

        // 节点数被 MAX_NODES 截断。
        let many: Vec<_> = (0..100_000)
            .map(|_| json!({ "shape": "cube", "params": { "size": 1.0 } }))
            .collect();
        let big = json!({ "nodes": many });
        assert_eq!(parse_scene(&big).len(), MAX_NODES);
    }

    #[test]
    fn non_finite_numbers_never_reach_the_buffer() {
        // NaN/inf 写进顶点缓冲会让任何读这个 GLB 的东西崩掉。get_f32 应退回默认值。
        // 1e400 是编译期就溢出的字面量，编不过；用运行时溢出到 inf 的值。
        let huge = 1e308_f64 * 10.0; // = inf
        let scene = json!({ "nodes": [
            { "shape": "sphere", "params": { "segments": 16, "radius": huge } }
        ]});
        let nodes = parse_scene(&scene);
        assert!(
            nodes[0].mesh.positions.iter().all(|v| v.iter().all(|x| x.is_finite())),
            "顶点里出现了非有限值",
        );
    }
}
