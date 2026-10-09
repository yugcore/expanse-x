use super::{RenderEncodeError, RenderPass};
use crate::{GraphicsPipeline, Sampler, TextureView};

/// One retained texture view/sampler pair. Slots are local to the draw's shader.
#[derive(Debug, Clone)]
pub struct TextureBinding {
    view: TextureView,
    sampler: Sampler,
}
impl TextureBinding {
    pub fn view(&self) -> &TextureView {
        &self.view
    }
    pub fn sampler(&self) -> &Sampler {
        &self.sampler
    }
}
impl RenderPass<'_> {
    /// Bind slot 0..8 after device checks. Failed binding preserves prior state.
    pub fn set_texture(
        &mut self,
        binding: u32,
        view: &TextureView,
        sampler: &Sampler,
    ) -> Result<(), RenderEncodeError> {
        if binding >= 8 {
            return Err(RenderEncodeError::InvalidTextureSlot { binding });
        }
        if !view.texture().belongs_to(&self.encoder.device) {
            return Err(RenderEncodeError::ForeignTexture {
                label: view.texture().label().to_owned(),
            });
        }
        if !sampler.belongs_to(&self.encoder.device) {
            return Err(RenderEncodeError::ForeignSampler {
                label: sampler.label().to_owned(),
            });
        }
        self.textures[binding as usize] = Some(TextureBinding {
            view: view.clone(),
            sampler: sampler.clone(),
        });
        Ok(())
    }
    pub(super) fn draw_resources(
        &self,
        pipeline: Option<&GraphicsPipeline>,
    ) -> Result<[Option<TextureBinding>; 8], RenderEncodeError> {
        if pipeline.is_some_and(|p| p.depth_state().is_some())
            && self.commands.depth_buffer().is_none()
        {
            return Err(RenderEncodeError::MissingDepthBuffer);
        }
        let mask = pipeline.map_or(0, |p| p.fragment_shader().required_textures());
        let mut retained = std::array::from_fn(|_| None);
        for (slot, target) in retained.iter_mut().enumerate() {
            if mask & (1 << slot) != 0 {
                *target = Some(
                    self.textures[slot]
                        .as_ref()
                        .ok_or(RenderEncodeError::MissingTextureBinding {
                            binding: slot as u32,
                        })?
                        .clone(),
                );
            }
        }
        Ok(retained)
    }
}
