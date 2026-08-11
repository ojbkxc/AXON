package com.axon.app;

import android.Manifest;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.os.Build;
import android.os.Bundle;
import android.webkit.WebChromeClient;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import androidx.appcompat.app.AppCompatActivity;
import androidx.core.app.ActivityCompat;
import androidx.core.content.ContextCompat;

public class MainActivity extends AppCompatActivity {
    private WebView webView;
    private static final int REQ_NOTIF = 100;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        requestNotificationPermission();

        Intent svc = new Intent(this, AxonService.class);
        startForegroundService(svc);

        webView = new WebView(this);
        WebSettings settings = webView.getSettings();
        settings.setJavaScriptEnabled(true);
        settings.setDomStorageEnabled(true);
        settings.setDatabaseEnabled(true);
        settings.setAllowFileAccess(false);
        settings.setAllowContentAccess(false);
        settings.setMixedContentMode(WebSettings.MIXED_CONTENT_COMPATIBILITY_MODE);
        webView.setWebViewClient(new WebViewClient());
        webView.setWebChromeClient(new WebChromeClient());
        setContentView(webView);

        new Thread(() -> {
            for (int i = 0; i < 60; i++) {
                try { Thread.sleep(200); } catch (InterruptedException e) { return; }
                try {
                    java.net.HttpURLConnection c =
                        (java.net.HttpURLConnection) new java.net.URL("http://127.0.0.1:8080/healthz").openConnection();
                    c.setConnectTimeout(500);
                    c.setReadTimeout(500);
                    c.connect();
                    int code = c.getResponseCode();
                    c.disconnect();
                    if (code == 200) {
                        runOnUiThread(() -> webView.loadUrl("http://127.0.0.1:8080/ui/"));
                        return;
                    }
                } catch (Exception ignored) { }
            }
            runOnUiThread(() -> webView.loadDataWithBaseURL(null,
                "<h2>AXON service not ready</h2><p>The native binary may have failed to start. Check logcat (tag: AxonService).</p>",
                "text/html", "utf-8", null));
        }, "axon-healthcheck").start();
    }

    private void requestNotificationPermission() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            if (ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS)
                    != PackageManager.PERMISSION_GRANTED) {
                ActivityCompat.requestPermissions(this,
                    new String[]{Manifest.permission.POST_NOTIFICATIONS}, REQ_NOTIF);
            }
        }
    }

    @Override
    public void onBackPressed() {
        if (webView != null && webView.canGoBack()) webView.goBack();
        else super.onBackPressed();
    }

    @Override
    protected void onDestroy() {
        if (webView != null) {
            webView.destroy();
            webView = null;
        }
        super.onDestroy();
    }
}
