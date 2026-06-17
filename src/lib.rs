use log::{error, info};

#[cfg(target_os = "android")]
use openxr;
#[cfg(target_os = "android")]
use space_soup::{XrContext, VkContext, Headset, Controllers, HandTrackers};
#[cfg(target_os = "android")]
use space_soup::renderer::xr_renderer::XrRenderer;
#[cfg(target_os = "android")]
use space_soup::renderer::{Cuboid, Color3};
#[cfg(target_os = "android")]
use glam::{Vec3, Quat};
#[cfg(target_os = "android")]
use mirror::{PosePacket, sender};

#[cfg(target_os = "android")]
#[no_mangle]
pub unsafe extern "C" fn ANativeActivity_onCreate(
    activity:         *mut std::ffi::c_void,
    saved_state:      *mut std::ffi::c_void,
    saved_state_size: usize,
) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Debug)
            .with_tag("quest_app"),
    );
    info!("ANativeActivity_onCreate started");

    let activity    = activity as usize;
    let saved_state = saved_state as usize;

    std::thread::Builder::new()
        .name("xr_main".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            ndk_glue::init(activity as _, saved_state as _, saved_state_size, || {
                run();
            });
        })
        .expect("failed to spawn xr_main");
}

pub fn run() {
    match run_inner() {
        Ok(())  => info!("App exited cleanly"),
        Err(e)  => error!("App error: {e}"),
    }
}

#[cfg(not(target_os = "android"))]
fn run_inner() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[cfg(target_os = "android")]
fn run_inner() -> Result<(), Box<dyn std::error::Error>> {
    let xr              = XrContext::new()?;
    let vk              = VkContext::new(&xr)?;
    let mut headset     = Headset::new(&xr, &vk)?;
    let mut controllers = Controllers::new(&xr.instance, &headset.session)?;
    let mut hands       = HandTrackers::new(&xr, &headset.session)?;
    let mut renderer    = XrRenderer::new(&vk, &xr, &headset.session)?;

    let mut mirror_stream: Option<std::net::TcpStream> = None;

    let cuboids = vec![
        Cuboid::solid(Vec3::new( 0.0, 0.5, -2.0), Vec3::splat(0.5),      Color3(220, 60,  60,  255)),
        Cuboid::wireframe(Vec3::new( 1.5, 0.5, -2.0), Vec3::splat(0.5),  Color3(60,  220, 60,  255)),
        Cuboid::solid_and_wire(
            Vec3::new(-1.5, 0.5, -2.0),
            Vec3::new(0.4, 0.8, 0.4),
            Color3(60, 100, 220, 200),
            Color3(200, 220, 255, 255),
        ),
        Cuboid::solid(
            Vec3::new(0.0, -0.05, -2.0),
            Vec3::new(10.0, 0.05, 10.0),
            Color3(60, 60, 60, 255),
        ),
    ];

    info!("All resources ready — entering event loop");

    let mut exit             = false;
    let mut frame_count:     u64 = 0;
    let mut input_log_timer: u64 = 0;
    let mut reconnect_timer: u64 = 0;

    'main: loop {
        if mirror_stream.is_none() {
            reconnect_timer += 1;
            if reconnect_timer >= 60 {
                reconnect_timer = 0;
                info!("Mirror: attempting connect to 127.0.0.1:7777...");
                match std::net::TcpStream::connect("127.0.0.1:7777") {
                    Ok(s) => {
                        info!("Mirror connected");
                        mirror_stream = Some(s);
                    }
                    Err(e) => {
                        info!("Mirror connect failed: {e}");
                    }
                }
            }
        }

        let mut event_buf = openxr::EventDataBuffer::new();
        loop {
            match xr.instance.poll_event(&mut event_buf)? {
                Some(openxr::Event::SessionStateChanged(e)) => {
                    if headset.handle_state_change(e.state())? { exit = true; }
                }
                Some(openxr::Event::InstanceLossPending(_)) => exit = true,
                Some(_) => {}
                None    => break,
            }
        }

        if exit { break 'main; }
        if !headset.running {
            std::thread::sleep(std::time::Duration::from_millis(100));
            continue;
        }

        let frame_state = headset.frame_waiter.wait()?;
        headset.frame_stream.begin()?;

        let time = frame_state.predicted_display_time;

        controllers.sync(&headset.session, &headset.stage, time)?;
        hands.sync(&headset.stage, time)?;

        input_log_timer += 1;
        if input_log_timer >= 90 {
            input_log_timer = 0;
            controllers.log();
            hands.log();
        }

        if !frame_state.should_render {
            headset.frame_stream.end(time, openxr::EnvironmentBlendMode::OPAQUE, &[])?;
            continue;
        }

        let (_, eye_views) = headset.session.locate_views(
            openxr::ViewConfigurationType::PRIMARY_STEREO,
            time,
            &headset.stage,
        )?;

        if let Some(ref mut stream) = mirror_stream {
            if let Some(ev) = eye_views.first() {
                let p = ev.pose.position;
                let o = ev.pose.orientation;
                let packet = PosePacket::new(
                    Vec3::new(p.x, p.y, p.z),
                    Quat::from_xyzw(o.x, o.y, o.z, o.w),
                );
                if sender::send(stream, &packet).is_err() {
                    info!("Mirror disconnected — will retry");
                    mirror_stream = None;
                    reconnect_timer = 0;
                }
            }
        }

        let proj_views = renderer.render_frame(
            &headset.session, &headset.stage, time, &cuboids,
        )?;
        let proj_layer = openxr::CompositionLayerProjection::new()
            .space(&headset.stage)
            .views(&proj_views);
        headset.frame_stream.end(
            time, openxr::EnvironmentBlendMode::OPAQUE, &[&proj_layer],
        )?;

        frame_count += 1;
        if frame_count % 500 == 0 {
            info!("Frame {frame_count}");
        }
    }

    renderer.cleanup();
    Ok(())
}