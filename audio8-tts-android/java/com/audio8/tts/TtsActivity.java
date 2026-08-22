package com.audio8.tts;

import android.app.NativeActivity;
import android.content.Intent;
import android.os.Bundle;

/**
 * NativeActivity subclass so we can apply immersive mode before the first frame and
 * receive ACTION_GET_CONTENT results for reference-voice audio.
 */
public class TtsActivity extends NativeActivity {

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        Immersive.apply(this);
        super.onCreate(savedInstanceState);
        Immersive.apply(this);
    }

    @Override
    protected void onResume() {
        super.onResume();
        Immersive.apply(this);
    }

    @Override
    public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus);
        if (hasFocus) {
            Immersive.apply(this);
        }
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        FilePicker.onActivityResult(this, requestCode, resultCode, data);
    }
}
