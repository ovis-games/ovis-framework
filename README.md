# Ovis Framework
This framework aims to generalize the [Entity Component System (ECS)](https://en.wikipedia.org/wiki/Entity_component_system) paradigm to the GPU by making resources like vertex buffers, textures, render targets, etc. first class components.
In general, the proposed ECS uses sparse sets for alls its components as described in [this article](https://skypjack.github.io/2019-03-07-ecs-baf-part-2/).
This framework, mirrors the data of both, the packed and sparse array, to buffers on the GPU which makes them available during rendering or GPGPU tasks.
The goal of this framework is to create an extensible environment that makes it very easy to prototype new rendering strategies.

## Custom Programming Language
When using graphics APIs you usually have to deal with two different programming languages: the language the application logic itself is written in and the language used for writing shaders.
In this case, the framework itself is written in [Rust](https://www.rust-lang.org/) and the the shaders are written in [WGSL](https://gpuweb.github.io/gpuweb/wgsl/).
Thus, every data type or programming logic you want to share between the CPU and GPU needs to be duplicated for both languages with special attention  how data is laid out in both of them.
To simplify this I propose to use a [custom programming language](https://github.com/ovis-games/ovis-runtime) I developed that transpiles to Rust and WGSL.
This way, everything can be easily shared between the GPU and CPU.


```
// This defines a struct that can be attached to entities as a component
#EntityComponent
struct Transform {
  position: Vec3F
  rotation: Quaternion
  scaling: Vec3F
}

// This defines a type alias that also defines an entity component. A type alias
// behaves exactly as the underlying type, but has special meaning when used as an
// input or output of a job (see below).
#EntityComponent
type LocalToWorld = Mat4x4F

// This function defines a job that gets executed on every update, e.g., every tick.
// It takes the entity component `Transform` as an input and produces another entity
// component `LocalToWorld`. This means, this function gets executed for every entity
// that contains the `Transform` component.
#Update
function updateLocalToWorldMatrices(transform: Transform) -> LocalToWorld {
  return Mat4F.createTransformMatrix(transform.position, transform.rotation, transform.scaling)
}

// This is another type alias that defines an entity component. The keyword List,
// indicates that every entity may have a list of this component.
#EntityComponent(List)
type VertexPosition = Vec3F

// This struct defines another entity component that defines the material of the entity.
// In this example, the material only contains a color.
#EntityComponent
struct BasicMaterial {
  color: Color
}

// The following defines a render job that is executed automatically for all entities that contain
// the components `LocalToWorld`, `BasicMaterial` and a list of `VertexPosition`s. Because this 
// job reads from the component `LocalToWorld` as an input and the job `updateLocalToWorldMatrices`
// writes to it, there will be an implicit dependency added to the scheduler. In addition, the
// framework will upload all changes to `LocalToWorld` (and also `VertexPosition` and `BasicMaterial`)
// to the corresponding GPU buffers before executing this render job.
// Note: `VertexShaderPosition` is a builtin resource that corresponds to WGSL's `@builtin(position)`
#VertexShader
function drawSomething(position: VertexPosition, localToWorld: LocalToWorld) -> VertexShaderPosition {
  return localToWorld.transformPosition(position)
}
#FragmentShader
function drawSomething(material: BasicMaterial) -> Color {
  return material.color
}

```

## Deferred Rendering Example
To show the flexibility of this system, this example shows how a simple deferred renderer can be implemented in the proposed framework.
```
#ViewportComponent
type Albedo = RenderTarget

#ViewportComponent
type NormalDepth = RenderTarget

#ViewportComponent
struct DeferredRendering {}

// This job will create the render targets for all viewports that should use deferred rendering
// i.e., that have the `DeferredRendering` component. It will get executed whenever any of the
// inputs change, i.e., whenever the dimensions of the viewport change.
#Update
function init(dimensions: ViewportDimensions, dr: DeferredRendering) -> (Albedo, Normal) {
  let albedo = RenderTarget()
  albedo.size = dimensions
  albedo.format = TextureFormat.RedGreenBlue32Float

  let normal = RenderTarget()
  normal.size = dimensions
  normal.format = TextureFormat.RedGreenBlue32Float

  return (albedo, normal)
}

#EntityComponent
type MaterialColor = Color

struct VertexToFragment {
  position: VertexShaderPosition
  normal: Vec2F
}

#VertexShader
function deferred(p: VertexPosition, n: VertexNormal, l2v: LocalToView) -> VertexToFragment {
  let v2f = VertexToFragment()
  v2f.position = l2v.transformPosition(p)
  v2f.normal = l2v.transformDirection(n)
  return v2f
}

#FragmentShader
function deferred(v2f: VertexToFragment, color: MaterialColor) -> (Albedo, Normal) {
  return (color, v2f.normal)
}

#EntityComponent
struct DirectionalLight {
  color: Color
}

// For rendering jobs that do not have any inputs marked as `List` we need to specify the Count.
// Also this draw call uses triangle strips, instead of triangle lists, which is the default.
// VertexIndex is also a builtin resource that corresponds to the `@builtin(vertex_index)`.
#VertexShader(TriangleStrip, Count = 4)
function directionalLight(index: VertexIndex) -> VertexShaderPosition {
  let positions = [
    Vec3F.createWith(-1.0,  1.0, 0.0),
    Vec3F.createWith( 1.0,  1.0, 0.0),
    Vec3F.createWith(-1.0, -1.0, 0.0),
    Vec3F.createWith( 1.0, -1.0, 0.0),
  ]
  return positions[index]
}
#FragmentShader(BlendOperation = Add)
function directionalLight(pos: VertexShaderPosition, l2W: LocalToWorld, dl: DirectionalLight, a: Albdeo, n: Normal) -> FrameBuffer {
  let texCoords = pos.xy * 0.5 + 0.5
	let albeo = a.sample(texCoords)
	let normal = n.sample(texCoords)
  
  // Assume that the light from an unrotated directional light comes from above
  let lightDirection = l2w.transformDirection(Vec3.createWith(0.0, 1.0, 0.0))
  let nDotL = Float.clamp(Vec3F.dot(lightDirection, normal), 0.0, 1.0)
  
	return nDotL * albedo * dl.color
}

```
