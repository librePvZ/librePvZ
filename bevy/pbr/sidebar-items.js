window.SIDEBAR_ITEMS = {"constant":["CLUSTERED_FORWARD_HANDLE","CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT","FOG_SHADER_HANDLE","FORWARD_IO_HANDLE","LIGHTMAP_SHADER_HANDLE","LIGHT_PROBE_SHADER_HANDLE","MAX_CASCADES_PER_LIGHT","MAX_DIRECTIONAL_LIGHTS","MAX_JOINTS","MAX_UNIFORM_BUFFER_CLUSTERABLE_OBJECTS","MAX_VIEW_LIGHT_PROBES","MESH_BINDINGS_HANDLE","MESH_FUNCTIONS_HANDLE","MESH_PIPELINE_VIEW_LAYOUT_SAFE_MAX_TEXTURES","MESH_PREPROCESS_SHADER_HANDLE","MESH_SHADER_HANDLE","MESH_TYPES_HANDLE","MESH_VIEW_BINDINGS_HANDLE","MESH_VIEW_TYPES_HANDLE","MORPH_HANDLE","PARALLAX_MAPPING_SHADER_HANDLE","PBR_AMBIENT_HANDLE","PBR_BINDINGS_SHADER_HANDLE","PBR_DEFERRED_FUNCTIONS_HANDLE","PBR_DEFERRED_TYPES_HANDLE","PBR_FRAGMENT_HANDLE","PBR_FUNCTIONS_HANDLE","PBR_LIGHTING_HANDLE","PBR_PREPASS_FUNCTIONS_SHADER_HANDLE","PBR_PREPASS_SHADER_HANDLE","PBR_SHADER_HANDLE","PBR_TRANSMISSION_HANDLE","PBR_TYPES_SHADER_HANDLE","PREPASS_BINDINGS_SHADER_HANDLE","PREPASS_IO_SHADER_HANDLE","PREPASS_SHADER_HANDLE","PREPASS_UTILS_SHADER_HANDLE","RGB9E5_FUNCTIONS_HANDLE","SHADOWS_HANDLE","SHADOW_SAMPLING_HANDLE","SKINNING_HANDLE","UTILS_HANDLE","VIEW_TRANSFORMATIONS_SHADER_HANDLE","VOLUMETRIC_FOG_HANDLE"],"enum":["ClusterConfig","ClusterFarZMode","FogFalloff","GpuClusterableObjects","LightEntity","OpaqueRendererMethod","ParallaxMappingMethod","RenderMeshInstanceGpuQueue","RenderMeshInstances","ScreenSpaceAmbientOcclusionQualityLevel","ShadowFilteringMethod","SimulationLightSystems","UvChannel"],"fn":["add_clusters","alpha_mode_pipeline_key","build_directional_light_cascades","calculate_cluster_factors","check_light_mesh_visibility","clear_directional_light_cascades","extract_camera_previous_view_data","extract_clusters","extract_lights","extract_meshes_for_cpu_building","extract_meshes_for_gpu_building","extract_skins","extract_volumetric_fog","generate_view_layouts","get_bind_group_layout_entries","get_bindings","prepare_clusters","prepare_fog","prepare_lights","prepare_mesh_bind_group","prepare_mesh_view_bind_groups","prepare_prepass_view_bind_group","prepare_preprocess_bind_groups","prepare_preprocess_pipelines","prepare_previous_view_uniforms","prepare_skins","prepare_ssr_pipelines","prepare_ssr_settings","prepare_view_depth_textures_for_volumetric_fog","prepare_volumetric_fog_pipelines","prepare_volumetric_fog_uniforms","queue_material_meshes","queue_prepass_material_meshes","queue_shadows","screen_space_specular_transmission_pipeline_key","setup_morph_and_skinning_defs","tonemapping_pipeline_key","update_directional_light_frusta","update_mesh_previous_global_transforms","update_point_light_frusta","update_previous_view_data","update_spot_light_frusta","write_mesh_culling_data_buffer"],"mod":["deferred","environment_map","graph","irradiance_volume","prelude","wireframe"],"struct":["AmbientLight","AtomicMaterialBindGroupId","Cascade","CascadeShadowConfig","CascadeShadowConfigBuilder","Cascades","CascadesVisibleEntities","ClusterZConfig","Clusters","CubemapVisibleEntities","DefaultOpaqueRendererMethod","DirectionalLight","DirectionalLightBundle","DirectionalLightShadowMap","DrawMesh","ExtendedMaterial","ExtractMeshesSet","ExtractedClusterConfig","ExtractedClusterableObjects","ExtractedDirectionalLight","ExtractedPointLight","FogMeta","FogPlugin","FogSettings","GlobalClusterableObjectMeta","GlobalVisibleClusterableObjects","GpuClusterableObject","GpuClusterableObjectsStorage","GpuClusterableObjectsUniform","GpuDirectionalCascade","GpuDirectionalLight","GpuFog","GpuLights","GpuMeshPreprocessPlugin","GpuPreprocessNode","LightMeta","LightProbe","LightProbePlugin","LightProbesBuffer","LightProbesUniform","Lightmap","LightmapPlugin","MaterialBindGroupId","MaterialExtensionKey","MaterialExtensionPipeline","MaterialMeshBundle","MaterialPipeline","MaterialPipelineKey","MaterialPlugin","MaterialProperties","MeshBindGroupPair","MeshBindGroups","MeshCullingData","MeshCullingDataBuffer","MeshFlags","MeshInputUniform","MeshLayouts","MeshPipeline","MeshPipelineKey","MeshPipelineViewLayout","MeshPipelineViewLayoutKey","MeshPipelineViewLayouts","MeshRenderPlugin","MeshTransforms","MeshUniform","MeshViewBindGroup","NotShadowCaster","NotShadowReceiver","PbrPlugin","PbrProjectionPlugin","PointLight","PointLightBundle","PointLightShadowMap","PreparedMaterial","PrepassPipeline","PrepassPipelinePlugin","PrepassPlugin","PrepassViewBindGroup","PreprocessBindGroup","PreprocessPipeline","PreprocessPipelineKey","PreprocessPipelines","PreviousGlobalTransform","RenderLightmaps","RenderMeshInstanceCpu","RenderMeshInstanceFlags","RenderMeshInstanceGpu","RenderMeshInstanceGpuBuilder","RenderMeshInstanceShared","RenderMeshInstancesCpu","RenderMeshInstancesGpu","RenderMeshQueueData","RenderViewLightProbes","ScreenSpaceAmbientOcclusionBundle","ScreenSpaceAmbientOcclusionPlugin","ScreenSpaceAmbientOcclusionSettings","ScreenSpaceAmbientOcclusionTextures","ScreenSpaceReflectionsBuffer","ScreenSpaceReflectionsBundle","ScreenSpaceReflectionsNode","ScreenSpaceReflectionsPipeline","ScreenSpaceReflectionsPipelineId","ScreenSpaceReflectionsPipelineKey","ScreenSpaceReflectionsPlugin","ScreenSpaceReflectionsSettings","ScreenSpaceReflectionsUniform","SetMaterialBindGroup","SetMeshBindGroup","SetMeshViewBindGroup","SetPrepassViewBindGroup","Shadow","ShadowBinKey","ShadowPassNode","ShadowSamplers","ShadowView","SkinIndices","SkinUniforms","SpotLight","SpotLightBundle","StandardMaterial","StandardMaterialFlags","StandardMaterialKey","StandardMaterialUniform","TransmittedShadowReceiver","ViewClusterBindings","ViewFogUniformOffset","ViewLightEntities","ViewLightProbesUniformOffset","ViewLightsUniformOffset","ViewScreenSpaceReflectionsUniformOffset","ViewShadowBindings","ViewVolumetricFogPipeline","ViewVolumetricFogUniformOffset","VisibleClusterableObjects","VolumetricFogNode","VolumetricFogPipeline","VolumetricFogPipelineKey","VolumetricFogPlugin","VolumetricFogSettings","VolumetricFogUniform","VolumetricFogUniformBuffer","VolumetricLight"],"trait":["LightProbeComponent","Material","MaterialExtension"],"type":["DrawPrepass","PbrBundle","RenderMaterialInstances","WithLight"]};