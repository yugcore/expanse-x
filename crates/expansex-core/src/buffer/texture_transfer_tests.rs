use super::*;
use crate::*;
use std::sync::Arc;

fn image(d: &Device) -> Texture {
    d.create_texture(TextureDescriptor {
        label: "image",
        width: 2,
        height: 2,
        format: TextureFormat::Rgba8Unorm,
        host_access: HostAccess::ReadWrite,
    })
    .unwrap()
}
fn buffer(d: &Device) -> Buffer {
    d.create_buffer(BufferDescriptor {
        label: "bytes",
        size: 16,
        usages: &[
            BufferUsage::TransferSource,
            BufferUsage::TransferDestination,
        ],
        host_access: HostAccess::ReadWrite,
    })
    .unwrap()
}
#[test]
fn poison_on_either_endpoint_preserves_failed_destination_and_completed_prefix() {
    for operation in 0..3 {
        for source_broken in [false, true] {
            let d = Instance::new().default_adapter().create_device();
            let a = image(&d);
            let b = image(&d);
            let bytes = buffer(&d);
            a.write_pixels(&[[11; 4]; 4]).unwrap();
            b.write_pixels(&[[22; 4]; 4]).unwrap();
            bytes.write(0, &[33; 16]).unwrap();
            let prefix = image(&d);
            let later = image(&d);
            let mut e = d.create_command_encoder();
            e.begin_render_pass(&prefix.as_framebuffer(), Some([44; 4]))
                .unwrap()
                .finish()
                .unwrap();
            let layout = TextureBufferLayout {
                offset: 0,
                bytes_per_row: 8,
            };
            match operation {
                0 => e.copy_buffer_to_texture(&bytes, layout, &a.view()).unwrap(),
                1 => e.copy_texture_to_buffer(&a.view(), &bytes, layout).unwrap(),
                _ => e.copy_texture(&a.view(), &b.view()).unwrap(),
            }
            e.begin_render_pass(&later.as_framebuffer(), Some([55; 4]))
                .unwrap()
                .finish()
                .unwrap();
            let broken_buffer =
                (operation == 0 && source_broken) || (operation == 1 && !source_broken);
            assert!(std::panic::catch_unwind(|| {
                if broken_buffer {
                    let _guard = bytes.lock_bytes().unwrap();
                    panic!("poison buffer");
                } else {
                    let broken = if operation == 2 && !source_broken {
                        &b
                    } else {
                        &a
                    };
                    let _guard = broken.state.pixels.lock().unwrap();
                    panic!("poison image");
                }
            })
            .is_err());
            let error = d.default_queue().submit(&e.finish()).unwrap_err();
            assert_eq!(
                error,
                QueueSubmitError::TextureTransferFailed {
                    command_index: 1,
                    source: TextureTransferExecutionError::StoragePoisoned {
                        role: if source_broken {
                            CopyBufferRole::Source
                        } else {
                            CopyBufferRole::Destination
                        },
                        kind: if broken_buffer {
                            ResourceKind::Buffer
                        } else {
                            ResourceKind::Texture
                        },
                        label: if broken_buffer {
                            "bytes".into()
                        } else {
                            "image".into()
                        },
                    }
                }
            );
            assert!(std::error::Error::source(&error).is_some());
            assert_eq!(prefix.as_framebuffer().pixel(0, 0).unwrap(), [44; 4]);
            assert_eq!(later.as_framebuffer().pixel(0, 0).unwrap(), [0; 4]);
            // Inspect poisoned guards only in tests; production never repairs poison.
            if operation == 1 {
                let guard = bytes.lock_bytes();
                if let Ok(guard) = guard {
                    assert_eq!(*guard, [33; 16]);
                } else {
                    assert_eq!(
                        *bytes.state.bytes.lock().unwrap_err().into_inner(),
                        [33; 16]
                    );
                }
            } else {
                let destination = if operation == 0 { &a } else { &b };
                let guard = destination
                    .state
                    .pixels
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                assert_eq!(*guard, [[if operation == 0 { 11 } else { 22 }; 4]; 4]);
            }
        }
    }
}
#[test]
fn transfers_retain_live_images_through_aliases_until_last_command_owner_drops() {
    let d = Instance::new().default_adapter().create_device();
    let a = image(&d);
    let b = image(&d);
    let bytes = buffer(&d);
    let wa = Arc::downgrade(&a.state);
    let wb = Arc::downgrade(&b.state);
    let mut e = d.create_command_encoder();
    let layout = TextureBufferLayout {
        offset: 0,
        bytes_per_row: 8,
    };
    e.copy_buffer_to_texture(&bytes, layout, &a.view()).unwrap();
    e.copy_texture(&a.as_framebuffer().as_texture().view(), &b.view())
        .unwrap();
    e.copy_texture_to_buffer(&b.view(), &bytes, layout).unwrap();
    let c = e.finish();
    drop(a);
    drop(b);
    drop(bytes);
    assert!(wa.upgrade().is_some() && wb.upgrade().is_some());
    d.default_queue().submit(&c).unwrap();
    drop(c);
    assert!(wa.upgrade().is_none() && wb.upgrade().is_none());
}
