package com.audio8.tts;

import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;

public final class ClipboardHelper {

    private ClipboardHelper() {}

    public static void setText(Context context, String text) {
        if (context == null || text == null) {
            return;
        }
        ClipboardManager cm =
                (ClipboardManager) context.getSystemService(Context.CLIPBOARD_SERVICE);
        if (cm != null) {
            cm.setPrimaryClip(ClipData.newPlainText("audio8", text));
        }
    }
}
