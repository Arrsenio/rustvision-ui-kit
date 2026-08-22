//! JNI bridges for immersive mode, playback, clipboard, and the audio picker.
//! Desktop builds compile to no-ops so the same UI crate can run under eframe.

#[cfg(target_os = "android")]
mod android_impl {
    use jni::objects::{JByteArray, JObject, JValue};
    use jni::JavaVM;
    use std::sync::Mutex;
    use winit::platform::android::activity::AndroidApp;

    static APP: Mutex<Option<AndroidApp>> = Mutex::new(None);

    pub fn store_app(app: AndroidApp) {
        if let Ok(mut slot) = APP.lock() {
            *slot = Some(app);
        }
    }

    fn with_env<T>(f: impl FnOnce(&mut jni::JNIEnv, JObject) -> jni::errors::Result<T>) -> Option<T> {
        let app = APP.lock().ok()?.as_ref()?.clone();
        let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut _).ok()? };
        let mut env = vm.attach_current_thread().ok()?;
        let activity = unsafe { JObject::from_raw(app.activity_as_ptr() as *mut _) };
        let result = f(&mut env, activity);
        match result {
            Ok(v) => Some(v),
            Err(err) => {
                log::warn!("jni: {err}");
                let _ = env.exception_clear();
                None
            }
        }
    }

    pub fn apply_immersive() {
        let _ = with_env(|env, activity| {
            env.call_static_method(
                "com/audio8/tts/Immersive",
                "apply",
                "(Landroid/app/Activity;)V",
                &[JValue::Object(&activity)],
            )?;
            let _ = activity.into_raw();
            Ok(())
        });
    }

    pub fn play_wav(wav: &[u8]) {
        let _ = with_env(|env, activity| {
            let arr: JByteArray = env.byte_array_from_slice(wav)?;
            env.call_static_method(
                "com/audio8/tts/AudioPlayer",
                "playWav",
                "(Landroid/app/Activity;[B)V",
                &[JValue::Object(&activity), JValue::Object(&JObject::from(arr))],
            )?;
            let _ = activity.into_raw();
            Ok(())
        });
    }

    pub fn stop_playback() {
        let _ = with_env(|env, activity| {
            env.call_static_method(
                "com/audio8/tts/AudioPlayer",
                "stop",
                "(Landroid/app/Activity;)V",
                &[JValue::Object(&activity)],
            )?;
            let _ = activity.into_raw();
            Ok(())
        });
    }

    pub fn is_playing() -> bool {
        with_env(|env, activity| {
            let v = env.call_static_method("com/audio8/tts/AudioPlayer", "isPlaying", "()Z", &[])?;
            let _ = activity.into_raw();
            Ok(v.z().unwrap_or(false))
        })
        .unwrap_or(false)
    }

    pub fn save_wav(wav: &[u8], name: &str) -> Option<String> {
        with_env(|env, activity| {
            let arr: JByteArray = env.byte_array_from_slice(wav)?;
            let jname = env.new_string(name)?;
            let v = env.call_static_method(
                "com/audio8/tts/AudioPlayer",
                "saveWav",
                "(Landroid/app/Activity;[BLjava/lang/String;)Ljava/lang/String;",
                &[
                    JValue::Object(&activity),
                    JValue::Object(&JObject::from(arr)),
                    JValue::Object(&JObject::from(jname)),
                ],
            )?;
            let obj = v.l()?;
            let path = env.get_string((&obj).into())?.to_string_lossy().into_owned();
            let _ = activity.into_raw();
            Ok(path)
        })
        .filter(|s| !s.is_empty())
    }

    pub fn copy_text(text: &str) {
        let _ = with_env(|env, activity| {
            let jtext = env.new_string(text)?;
            env.call_static_method(
                "com/audio8/tts/ClipboardHelper",
                "setText",
                "(Landroid/content/Context;Ljava/lang/String;)V",
                &[JValue::Object(&activity), JValue::Object(&JObject::from(jtext))],
            )?;
            let _ = activity.into_raw();
            Ok(())
        });
    }

    pub fn pick_audio() {
        let _ = with_env(|env, activity| {
            env.call_static_method(
                "com/audio8/tts/FilePicker",
                "pickAudio",
                "(Landroid/app/Activity;)V",
                &[JValue::Object(&activity)],
            )?;
            let _ = activity.into_raw();
            Ok(())
        });
    }

    pub fn take_picked_audio() -> Option<(Vec<u8>, String)> {
        with_env(|env, activity| {
            let bytes_v = env.call_static_method(
                "com/audio8/tts/FilePicker",
                "takeBytes",
                "()[B",
                &[],
            )?;
            let obj = bytes_v.l()?;
            if obj.is_null() {
                let _ = activity.into_raw();
                return Ok(None);
            }
            let arr = JByteArray::from(obj);
            let rust_bytes = env.convert_byte_array(&arr)?;
            let name_v = env.get_static_field(
                "com/audio8/tts/FilePicker",
                "lastName",
                "Ljava/lang/String;",
            )?;
            let name_obj = name_v.l()?;
            let name = if name_obj.is_null() {
                "reference.wav".to_string()
            } else {
                env.get_string((&name_obj).into())?
                    .to_string_lossy()
                    .into_owned()
            };
            let _ = activity.into_raw();
            Ok(Some((rust_bytes, name)))
        })
        .flatten()
    }

    pub fn picker_error() -> Option<String> {
        with_env(|env, activity| {
            let v = env.get_static_field(
                "com/audio8/tts/FilePicker",
                "lastError",
                "Ljava/lang/String;",
            )?;
            let obj = v.l()?;
            let _ = activity.into_raw();
            if obj.is_null() {
                return Ok(None);
            }
            let s = env.get_string((&obj).into())?.to_string_lossy().into_owned();
            if s.is_empty() {
                Ok(None)
            } else {
                Ok(Some(s))
            }
        })
        .flatten()
    }
}

#[cfg(target_os = "android")]
pub use android_impl::*;

#[cfg(not(target_os = "android"))]
mod desktop_impl {
    pub fn apply_immersive() {}
    pub fn play_wav(_wav: &[u8]) {}
    pub fn stop_playback() {}
    pub fn is_playing() -> bool {
        false
    }
    pub fn save_wav(_wav: &[u8], _name: &str) -> Option<String> {
        None
    }
    pub fn copy_text(_text: &str) {}
    pub fn pick_audio() {}
    pub fn take_picked_audio() -> Option<(Vec<u8>, String)> {
        None
    }
    pub fn picker_error() -> Option<String> {
        None
    }
}

#[cfg(not(target_os = "android"))]
pub use desktop_impl::*;
