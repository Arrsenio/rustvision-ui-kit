package com.audio8.tts;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;
import java.io.ByteArrayOutputStream;
import java.io.InputStream;

public final class FilePicker {

    public static final int REQUEST_AUDIO = 0xA801;

    public static volatile byte[] lastBytes;
    public static volatile String lastName;
    public static volatile String lastError;

    private FilePicker() {}

    public static void pickAudio(Activity activity) {
        if (activity == null) {
            return;
        }
        lastError = null;
        Intent intent = new Intent(Intent.ACTION_GET_CONTENT);
        intent.setType("audio/*");
        intent.addCategory(Intent.CATEGORY_OPENABLE);
        intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
        activity.startActivityForResult(
                Intent.createChooser(intent, "Reference audio"), REQUEST_AUDIO);
    }

    public static void onActivityResult(
            Activity activity, int requestCode, int resultCode, Intent data) {
        if (requestCode != REQUEST_AUDIO) {
            return;
        }
        if (resultCode != Activity.RESULT_OK || data == null) {
            lastError = "PICK CANCELLED";
            return;
        }
        Uri uri = data.getData();
        if (uri == null) {
            lastError = "NO URI";
            return;
        }
        try {
            InputStream in = activity.getContentResolver().openInputStream(uri);
            if (in == null) {
                lastError = "OPEN FAILED";
                return;
            }
            ByteArrayOutputStream out = new ByteArrayOutputStream();
            byte[] buf = new byte[8192];
            int n;
            while ((n = in.read(buf)) > 0) {
                out.write(buf, 0, n);
            }
            in.close();
            lastBytes = out.toByteArray();
            lastName = uri.getLastPathSegment();
            lastError = null;
        } catch (Exception e) {
            lastError = String.valueOf(e.getMessage());
        }
    }

    public static byte[] takeBytes() {
        byte[] bytes = lastBytes;
        lastBytes = null;
        return bytes;
    }
}
