package com.audio8.tts;

import android.app.Activity;
import android.media.MediaPlayer;
import android.os.Environment;
import java.io.File;
import java.io.FileOutputStream;

public final class AudioPlayer {

    private static MediaPlayer player;
    private static File lastFile;

    private AudioPlayer() {}

    public static void playWav(final Activity activity, final byte[] wav) {
        if (activity == null || wav == null || wav.length == 0) {
            return;
        }
        activity.runOnUiThread(
                new Runnable() {
                    @Override
                    public void run() {
                        try {
                            stopInternal();
                            File dir = activity.getCacheDir();
                            File tmp = File.createTempFile("audio8_", ".wav", dir);
                            FileOutputStream fos = new FileOutputStream(tmp);
                            fos.write(wav);
                            fos.close();
                            lastFile = tmp;
                            MediaPlayer mp = new MediaPlayer();
                            mp.setDataSource(tmp.getAbsolutePath());
                            mp.setOnCompletionListener(
                                    new MediaPlayer.OnCompletionListener() {
                                        @Override
                                        public void onCompletion(MediaPlayer mediaPlayer) {
                                            stopInternal();
                                        }
                                    });
                            mp.prepare();
                            mp.start();
                            player = mp;
                        } catch (Exception ignored) {
                            stopInternal();
                        }
                    }
                });
    }

    public static void stop(Activity activity) {
        if (activity == null) {
            stopInternal();
            return;
        }
        activity.runOnUiThread(
                new Runnable() {
                    @Override
                    public void run() {
                        stopInternal();
                    }
                });
    }

    public static boolean isPlaying() {
        try {
            return player != null && player.isPlaying();
        } catch (Exception e) {
            return false;
        }
    }

    public static String saveWav(Activity activity, byte[] wav, String name) {
        if (activity == null || wav == null || wav.length == 0) {
            return "";
        }
        try {
            File dir = activity.getExternalFilesDir(Environment.DIRECTORY_MUSIC);
            if (dir == null) {
                dir = activity.getFilesDir();
            }
            if (!dir.exists() && !dir.mkdirs()) {
                return "";
            }
            String safe = name == null || name.isEmpty() ? "audio8.wav" : name;
            File out = new File(dir, safe);
            FileOutputStream fos = new FileOutputStream(out);
            fos.write(wav);
            fos.close();
            return out.getAbsolutePath();
        } catch (Exception e) {
            return "";
        }
    }

    private static void stopInternal() {
        MediaPlayer mp = player;
        player = null;
        if (mp != null) {
            try {
                if (mp.isPlaying()) {
                    mp.stop();
                }
            } catch (Exception ignored) {
            }
            try {
                mp.release();
            } catch (Exception ignored) {
            }
        }
        lastFile = null;
    }
}
