use std::borrow::Cow;

use bevy::{
    asset::{AssetServer, Handle},
    ecs::{component::Component, entity::Entity, query::With, world::World},
    log::warn,
    platform::collections::HashMap,
    render::{
        render_graph::Node,
        render_resource::{
            BufferInitDescriptor, BufferUsages, CachedComputePipelineId, CachedPipelineState,
            ComputePassDescriptor, ComputePipelineDescriptor, PipelineCache, ShaderStages,
        },
        renderer::RenderDevice,
    },
    shader::{PipelineCacheError, Shader},
};
use bytemuck::{AnyBitPattern, NoUninit};

use crate::{
    components::render::{BindGroupBuilder, SwappableBindGroup},
    render::double_buffer::DoubleBuffer,
};
const EXT_BG_WAIT_THRESHOLD: u32 = 120;

type BindGroupFinder = Box<dyn Fn(&World) -> Option<Entity> + Send + Sync>;
type BufferAdder =
    Box<dyn FnOnce(&RenderDevice, &mut BindGroupBuilder, Option<&str>) + Send + Sync>;
type MarkerInserter = Box<dyn FnOnce(&mut World, Entity) + Send + Sync>;

pub struct ComputePassBuilder {
    buffer_adders: Vec<BufferAdder>,
    entry_point: &'static str,
    external_bind_groups: HashMap<u32, BindGroupFinder>,
    label: Option<&'static str>,
    owned_marker: Option<MarkerInserter>,
    shader_path: Option<&'static str>,
    workgroups: Option<(u32, u32, u32)>,
}

impl ComputePassBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the label for the `ComputePass`.  Buffers will inherit `_buffer_i` label suffixes.
    #[must_use]
    pub fn label(mut self, label: &'static str) -> Self {
        self.label = Some(label);
        self
    }

    /// The path to the shader that will run this `ComputePass`
    #[must_use]
    pub fn shader(mut self, path: &'static str) -> Self {
        self.shader_path = Some(path);
        self
    }

    /// The name of the entry point in the shader
    #[must_use]
    pub fn entry_point(mut self, entry: &'static str) -> Self {
        self.entry_point = entry;
        self
    }

    /// The workgroup size for the compute shader run
    #[must_use]
    pub fn workgroups(mut self, x: u32, y: u32, z: u32) -> Self {
        self.workgroups = Some((x, y, z));
        self
    }

    /// Add a double buffer populated with the supplied data
    #[must_use]
    pub fn double_buffer<T: NoUninit + AnyBitPattern + Send + Sync + 'static>(
        mut self,
        data: Vec<T>,
    ) -> Self {
        self.buffer_adders.push(Box::new(
            move |render_device: &RenderDevice,
                  builder: &mut BindGroupBuilder,
                  label: Option<&str>| {
                let double_buffer = DoubleBuffer::new(render_device, &data, label);
                builder.add_compute_double(double_buffer);
            },
        ));
        self
    }

    #[must_use]
    pub fn buffer<T: NoUninit + Send + Sync + 'static>(
        mut self,
        data: Vec<T>,
        read_only: bool,
        usage: BufferUsages,
        visibility: ShaderStages,
    ) -> Self {
        self.buffer_adders.push(Box::new(
            move |render_device: &RenderDevice,
                  builder: &mut BindGroupBuilder,
                  label: Option<&str>| {
                let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                    label,
                    contents: bytemuck::cast_slice(&data),
                    usage,
                });
                builder.add_buffer(buffer, visibility, read_only);
            },
        ));
        self
    }

    /// Add a readonly buffer populated with the supplied data
    #[must_use]
    pub fn buffer_read<T: NoUninit + Send + Sync + 'static>(self, data: Vec<T>) -> Self {
        self.buffer(data, true, BufferUsages::STORAGE, ShaderStages::COMPUTE)
    }

    /// Add a buffer populated with the supplied data
    #[must_use]
    pub fn buffer_write<T: NoUninit + Send + Sync + 'static>(self, data: Vec<T>) -> Self {
        self.buffer(data, false, BufferUsages::STORAGE, ShaderStages::COMPUTE)
    }

    /// Sets the marker component to add to the owned bind group entity.
    /// This allows other passes to reference this bind group.
    #[must_use]
    pub fn owned_bind_group_marker<M: Component>(mut self, marker: M) -> Self {
        self.owned_marker = Some(Box::new(move |world: &mut World, entity: Entity| {
            world.entity_mut(entity).insert(marker);
        }));
        self
    }

    /// References an external bind group by marker component at the given slot.
    /// If called multiple times with the same slot, the value will be overwritten.
    #[must_use]
    pub fn bind_group<M: Component>(mut self, slot: u32) -> Self {
        self.external_bind_groups.insert(
            slot,
            Box::new(|world: &World| {
                let mut query =
                    world.try_query_filtered::<Entity, (With<SwappableBindGroup>, With<M>)>()?;
                query.iter(world).next()
            }),
        );
        self
    }

    /// Builds the compute pass, creating buffers and spawning the owned bind group entity.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - `shader` was not set
    /// - `workgroups` was not set
    /// - `owned_bind_group_marker` was set but no buffers were added
    #[must_use]
    pub fn build(self, world: &mut World) -> ComputePass {
        let label = self.label.unwrap_or("compute_pass");
        let shader_path = self.shader_path.expect("shader path is required");
        let workgroups = self.workgroups.expect("workgroups is required");

        assert!(
            !(self.owned_marker.is_some() && self.buffer_adders.is_empty()),
            "owned_bind_group_marker requires at least one buffer"
        );

        let render_device = world.resource::<RenderDevice>().clone();
        let asset_server = world.resource::<AssetServer>().clone();
        let shader = asset_server.load(shader_path);

        let owned_bind_group = if self.buffer_adders.is_empty() {
            None
        } else {
            let mut builder = BindGroupBuilder::new();
            for (i, adder) in self.buffer_adders.into_iter().enumerate() {
                let buffer_label = format!("{label}_buffer_{i}");
                adder(&render_device, &mut builder, Some(&buffer_label));
            }

            let swappable = builder.build(&render_device, Some(label));
            let entity = world.spawn(swappable).id();

            if let Some(marker_inserter) = self.owned_marker {
                marker_inserter(world, entity);
            }

            Some(entity)
        };

        let external_bind_groups = self.external_bind_groups;

        ComputePass {
            owned_bind_group,
            external_bind_groups,
            pipeline_id: None,
            shader,
            entry_point: Cow::Borrowed(self.entry_point),
            workgroups,
            state: ComputePassState::Loading { frames_waited: 0 },
            label,
        }
    }
}

impl Default for ComputePassBuilder {
    fn default() -> Self {
        Self {
            buffer_adders: Vec::new(),
            entry_point: "main",
            external_bind_groups: HashMap::new(),
            label: None,
            owned_marker: None,
            shader_path: None,
            workgroups: None,
        }
    }
}

pub enum ComputePassState {
    Loading { frames_waited: u32 },
    Ready,
}

#[derive(Component)]
pub struct ComputePass {
    owned_bind_group: Option<Entity>,
    external_bind_groups: HashMap<u32, BindGroupFinder>,
    pipeline_id: Option<CachedComputePipelineId>,
    shader: Handle<Shader>,
    entry_point: Cow<'static, str>,
    workgroups: (u32, u32, u32),
    state: ComputePassState,
    label: &'static str,
}

impl ComputePass {
    #[must_use]
    pub fn builder() -> ComputePassBuilder {
        ComputePassBuilder::new()
    }
}

impl Node for ComputePass {
    fn update(&mut self, world: &mut World) {
        if matches!(self.state, ComputePassState::Ready) {
            return;
        }

        let ComputePassState::Loading { frames_waited } = &mut self.state else {
            return;
        };

        if self.pipeline_id.is_none() {
            let mut layouts = Vec::new();
            if let Some(entity) = self.owned_bind_group {
                let bind_group = world
                    .get::<SwappableBindGroup>(entity)
                    .expect("owned bind group entity missing SwappableBindGroup");
                layouts.push((0, bind_group.layout().clone()));
            }

            for (slot, finder) in &self.external_bind_groups {
                let Some(entity) = finder(world) else {
                    *frames_waited += 1;
                    if *frames_waited > EXT_BG_WAIT_THRESHOLD {
                        warn!(
                            "ComputePass '{}': waiting for external bind group at slot {}",
                            self.label, slot
                        );
                    }
                    return;
                };
                let bind_group = world
                    .get::<SwappableBindGroup>(entity)
                    .expect("external bind group entity missing SwappableBindGroup");
                layouts.push((*slot, bind_group.layout().clone()));
            }

            layouts.sort_by_key(|(slot, _)| *slot);
            for (i, (slot, _)) in layouts.iter().enumerate() {
                assert!(
                    *slot == i as u32,
                    "ComputePass '{}': bind group slots must be contiguous starting from 0, found gap at slot {}",
                    self.label,
                    i
                );
            }

            let layout = layouts.into_iter().map(|(_, l)| l).collect();
            let pipeline_cache = world.resource::<PipelineCache>();
            let pipeline_id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some(Cow::Borrowed(self.label)),
                layout,
                shader: self.shader.clone(),
                shader_defs: vec![],
                entry_point: Some(self.entry_point.clone()),
                push_constant_ranges: vec![],
                zero_initialize_workgroup_memory: true,
            });
            self.pipeline_id = Some(pipeline_id);
        }

        let pipeline_cache = world.resource::<PipelineCache>();
        match pipeline_cache
            .get_compute_pipeline_state(self.pipeline_id.expect("pipeline id should be set"))
        {
            CachedPipelineState::Ok(_) => {
                self.state = ComputePassState::Ready;
            }
            CachedPipelineState::Err(PipelineCacheError::ShaderNotLoaded(_)) => {
                // Still loading shader
            }
            CachedPipelineState::Err(err) => {
                panic!("ComputePass '{}': pipeline error: {err}", self.label);
            }
            _ => {
                // Still loading
            }
        }
    }

    fn run<'w>(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        if !matches!(self.state, ComputePassState::Ready) {
            return Ok(());
        }

        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(pipeline) =
            pipeline_cache.get_compute_pipeline(self.pipeline_id.expect("to have a pipeline id"))
        else {
            return Ok(());
        };

        let mut pass =
            render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor {
                    label: Some(self.label),
                    timestamp_writes: None,
                });

        pass.set_pipeline(pipeline);

        if let Some(entity) = self.owned_bind_group {
            let bind_group = world
                .get::<SwappableBindGroup>(entity)
                .expect("to find bind group");
            pass.set_bind_group(0, bind_group.current(), &[]);
        }

        for (slot, finder) in &self.external_bind_groups {
            let entity = finder(world).expect("external bind group not found at runtime");
            let bind_group = world
                .get::<SwappableBindGroup>(entity)
                .expect("to find bind group");
            pass.set_bind_group(*slot, bind_group.current(), &[]);
        }

        pass.dispatch_workgroups(self.workgroups.0, self.workgroups.1, self.workgroups.2);
        Ok(())
    }
}
