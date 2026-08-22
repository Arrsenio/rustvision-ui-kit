package com.audio8.tts;

import android.app.Activity;
import android.os.Build;
import android.view.View;
import android.view.Window;
import android.view.WindowInsets;
import android.view.WindowInsetsController;

/**
 * Sticky immersive full-screen for the OLED look: no status bar, no navigation bar.
 *
 * View operations must happen on the UI thread, so everything is posted there. The
 * display cutout is deliberately left un-inset (no SHORT_EDGES) so the system keeps
 * content clear of the punch-hole camera; against an OLED-black UI the reserved strip
 * is invisible anyway.
 */
public final class Immersive {

    private Immersive() {}

    public static void apply(Activity activity) {
        if (activity == null) {
            return;
        }
        activity.runOnUiThread(
                new Runnable() {
                    @Override
                    public void run() {
                        hide(activity);
                    }
                });
    }

    private static void hide(Activity activity) {
        Window window = activity.getWindow();
        if (window == null) {
            return;
        }

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            hideBars(window);

            View decor = window.getDecorView();
            decor.setOnApplyWindowInsetsListener(
                    new View.OnApplyWindowInsetsListener() {
                        @Override
                        public WindowInsets onApplyWindowInsets(View view, WindowInsets insets) {
                            if (insets.isVisible(WindowInsets.Type.statusBars())
                                    || insets.isVisible(WindowInsets.Type.navigationBars())) {
                                view.post(
                                        new Runnable() {
                                            @Override
                                            public void run() {
                                                hideBars(window);
                                            }
                                        });
                            }
                            return view.onApplyWindowInsets(insets);
                        }
                    });
        } else {
            View decor = window.getDecorView();
            decor.setSystemUiVisibility(
                    View.SYSTEM_UI_FLAG_LAYOUT_STABLE
                            | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
                            | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
                            | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                            | View.SYSTEM_UI_FLAG_FULLSCREEN
                            | View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY);

            decor.setOnSystemUiVisibilityChangeListener(
                    new View.OnSystemUiVisibilityChangeListener() {
                        @Override
                        public void onSystemUiVisibilityChange(int visibility) {
                            if ((visibility & View.SYSTEM_UI_FLAG_FULLSCREEN) == 0) {
                                hide(activity);
                            }
                        }
                    });
        }
    }

    private static void hideBars(Window window) {
        window.setDecorFitsSystemWindows(false);
        WindowInsetsController controller = window.getInsetsController();
        if (controller != null) {
            controller.hide(WindowInsets.Type.systemBars());
            controller.setSystemBarsBehavior(
                    WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
        }
    }
}
