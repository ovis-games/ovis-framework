# Ovis Framework
This framework aims to generalize the [Entity Component System (ECS)](https://en.wikipedia.org/wiki/Entity_component_system) paradigm to the GPU by making GPU resources like vertex buffers, textures, render targets, etc. first class components.


In traditional implementations of ECS architectures systems can modify the application state by adding/removing/modifying entities or their components, however 

Todays implementations of ECS architectures aim to provide an efficient and cache-friendly way to store and update the state of an application.
The state is stored in the components that are attached to a set of entities and the state can be modified by a set of systems.
However, GPU resources such as vertex buffers, textures, and render targets are usually not considered components in themselves.

For example: when you want to render a mesh in a tradition ECS architecture the `Mesh` component would only store a reference, e.g., the filename, to the mesh.

