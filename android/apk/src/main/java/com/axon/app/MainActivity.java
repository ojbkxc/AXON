package com.axon.app;

import android.content.Intent;
import android.os.Bundle;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import androidx.appcompat.app.AppCompatActivity;

public class MainActivity extends AppCompatActivity {
    private WebView webView;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        Intent svc = new Intent(this, AxonService.class);
        startForegroundService(svc);

        webView = new WebView(this);
        webView.getSettings().setJavaScriptEnabled(true);
        webView.getSettings().setDomStorageEnabled(true);
        webView.setWebViewClient(new WebViewClient());
        setContentView(webView);

        new Thread(() -> {
            for (int i = 0; i < 50; i++) {
                try { Thread.sleep(200); } catch (InterruptedException e) { return; }
                try {
                    java.net.HttpURLConnection c =
                        (java.net.HttpURLConnection) new java.net.URL("http://127.0.0.1:8080/healthz").openConnection();
                    c.setConnectTimeout(500);
                    c.connect();
                    if (c.getResponseCode() == 200) {
                        runOnUiThread(() -> webView.loadUrl("http://127.0.0.1:8080/ui/"));
                        return;
                    }
                } catch (Exception ignored) { }
            }
            runOnUiThread(() -> webView.loadData(
                "<h2>AXON service not ready</h2><p>Check logs and config.</p>", "text/html", "utf-8"));
        }).start();
    }

    @Override
    public void onBackPressed() {
        if (webView.canGoBack()) webView.goBack();
        else super.onBackPressed();
    }
}
