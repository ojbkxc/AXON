package com.axon.app;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.content.pm.ServiceInfo;
import android.os.Build;
import android.os.IBinder;
import android.util.Log;

import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;

public class AxonService extends Service {
    private static final String TAG = "AxonService";
    private static final String CHANNEL = "axon-service";
    private static final int NOTIF_ID = 1;
    private Process axonProcess;

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        startForeground();
        new Thread(this::runAxon, "axon-runner").start();
        return START_STICKY;
    }

    private void runAxon() {
        try {
            File bin = findNativeBinary();
            File cfg = extractAsset("config.yaml", false);
            if (bin == null || !bin.exists() || !bin.canExecute()) {
                Log.e(TAG, "axon binary not found or not executable");
                return;
            }
            File workDir = getFilesDir();
            String configPath = (cfg != null) ? cfg.getAbsolutePath() : new File(workDir, "config.yaml").getAbsolutePath();

            Log.i(TAG, "launching axon: " + bin.getAbsolutePath() + " --config " + configPath);
            ProcessBuilder pb = new ProcessBuilder(
                bin.getAbsolutePath(),
                "--config", configPath,
                "--addr", "127.0.0.1:8080"
            );
            pb.directory(workDir);
            pb.redirectErrorStream(true);
            pb.environment().put("AXON_LOG_LEVEL", "info");
            pb.environment().put("RUST_LOG", "info");
            axonProcess = pb.start();

            InputStream is = axonProcess.getInputStream();
            byte[] buf = new byte[4096];
            int n;
            while ((n = is.read(buf)) != -1) {
                Log.i(TAG, new String(buf, 0, n));
            }
            int code = axonProcess.waitFor();
            Log.i(TAG, "axon exited with code " + code);
        } catch (Exception e) {
            Log.e(TAG, "failed to run axon", e);
        }
    }

    private File findNativeBinary() {
        String nativeDir = getApplicationInfo().nativeLibraryDir;
        File bin = new File(nativeDir, "libaxon.so");
        Log.i(TAG, "nativeLibraryDir=" + nativeDir + " binary=" + bin.getAbsolutePath());
        if (bin.exists() && bin.canExecute()) {
            return bin;
        }
        File legacy = new File(getFilesDir(), "axon");
        if (legacy.exists()) {
            Log.w(TAG, "using legacy binary from filesDir (may fail on Android 10+)");
            return legacy;
        }
        return bin;
    }

    private File extractAsset(String name, boolean executable) {
        try (InputStream in = getAssets().open(name)) {
            File out = new File(getFilesDir(), name);
            try (OutputStream os = new FileOutputStream(out)) {
                byte[] buf = new byte[8192];
                int n;
                while ((n = in.read(buf)) != -1) os.write(buf, 0, n);
            }
            if (executable) {
                try {
                    out.setExecutable(true, true);
                } catch (Exception ignored) { }
            }
            return out;
        } catch (Exception e) {
            Log.w(TAG, "asset " + name + " not found: " + e.getMessage());
            return null;
        }
    }

    private void startForeground() {
        NotificationManager nm = (NotificationManager) getSystemService(Context.NOTIFICATION_SERVICE);
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            NotificationChannel ch = new NotificationChannel(CHANNEL, "AXON", NotificationManager.IMPORTANCE_LOW);
            nm.createNotificationChannel(ch);
        }
        Notification notif = new Notification.Builder(this, CHANNEL)
            .setContentTitle("AXON")
            .setContentText("AXON gateway running")
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setOngoing(true)
            .build();
        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                startForeground(NOTIF_ID, notif, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC);
            } else {
                startForeground(NOTIF_ID, notif);
            }
        } catch (Exception e) {
            Log.e(TAG, "startForeground failed", e);
        }
    }

    @Override
    public void onDestroy() {
        if (axonProcess != null) {
            axonProcess.destroy();
            axonProcess = null;
        }
        super.onDestroy();
    }
}
