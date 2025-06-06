use crate::{ws, Result, context::MucoContextRef};
use warp::{http::StatusCode, Reply};

pub async fn ws_handler(ws: warp::ws::Ws, context_ref: MucoContextRef) -> Result<impl Reply> {
    Ok(ws.on_upgrade(move |socket| ws::frontend_connection_process(socket, context_ref)))
}

pub async fn health_handler() -> Result<impl Reply> {
    Ok(StatusCode::OK)
}

pub async fn trust_handler() -> Result<impl Reply> {
    let html = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>MUCO Server - Trust Certificate</title>
    <style>
        body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 600px;
            margin: 2rem auto;
            padding: 2rem;
            line-height: 1.6;
            background: #f5f5f5;
        }
        .container {
            background: white;
            padding: 2rem;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        .success {
            color: #16a34a;
            font-weight: bold;
            font-size: 1.2em;
            text-align: center;
            margin-bottom: 1rem;
        }
        .info {
            background: #e1f5fe;
            padding: 1rem;
            border-radius: 4px;
            border-left: 4px solid #0277bd;
            margin: 1rem 0;
        }
        .warning {
            background: #fff3cd;
            padding: 1rem;
            border-radius: 4px;
            border-left: 4px solid #ffc107;
            margin: 1rem 0;
        }
        .footer {
            text-align: center;
            margin-top: 2rem;
            color: #666;
            font-size: 0.9em;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="success">
            ✅ Certificate Trust Successful! (v2.0 - NO BUTTONS)
        </div>
        
        <div class="info">
            <strong>What just happened?</strong><br>
            By visiting this page over HTTPS, you've instructed your browser to trust the 
            self-signed certificate for this MUCO server. This allows the MUCO Manager 
            frontend to connect securely via WebSocket.
        </div>
        
        <div class="warning">
            <strong>Security Note:</strong><br>
            This is a self-signed certificate, which means it wasn't issued by a trusted 
            certificate authority. It provides encryption but not identity verification. 
            Only proceed if you trust this server.
        </div>
        
        <h3>What happens next:</h3>
        <ol>
            <li>This page will automatically test the WebSocket connection</li>
            <li>If successful, this tab will auto-close</li>
            <li>Return to MUCO Manager - connection will work automatically</li>
        </ol>
        
        <div id="test-result" style="margin-top: 1rem; text-align: center;">🔄 Waiting for connection attempt from MUCO Manager...</div>
        
        <div class="footer">
            MUCO Server Dynamic SSL Certificate<br>
            Generated automatically for secure connections
        </div>
    </div>

    <script>
        let isTestingConnection = false;
        
        function testWebSocket() {
            if (isTestingConnection) return; // Prevent multiple concurrent tests
            isTestingConnection = true;
            
            const result = document.getElementById('test-result');
            result.innerHTML = '🔄 Testing WebSocket connection...';
            result.style.color = '#0277bd';
            
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const wsUrl = `${protocol}//${window.location.host}/ws`;
            
            try {
                const ws = new WebSocket(wsUrl);
                
                ws.onopen = function() {
                    result.innerHTML = '✅ WebSocket connection successful! Closing tab in 2 seconds...';
                    result.style.color = '#16a34a';
                    ws.close();
                    
                    console.log('WebSocket opened successfully!');
                    
                    // Immediately signal success to parent window
                    try {
                        if (window.opener) {
                            console.log('Sending trust-complete message to parent window');
                            window.opener.postMessage({ type: 'trust-complete' }, '*');
                        } else {
                            console.log('No window.opener found');
                        }
                        
                        // Close tab after success
                        console.log('Attempting to close tab in 2 seconds...');
                        setTimeout(() => {
                            console.log('Closing tab now...');
                            window.close();
                        }, 2000);
                    } catch (e) {
                        console.log('Auto-close failed:', e);
                        result.innerHTML = '✅ WebSocket connection successful! You can manually close this tab.';
                    }
                    isTestingConnection = false;
                };
                
                ws.onerror = function() {
                    result.innerHTML = '🔄 Waiting for connection attempt from MUCO Manager...';
                    result.style.color = '#666';
                    isTestingConnection = false;
                };
                
                setTimeout(() => {
                    if (ws.readyState === WebSocket.CONNECTING) {
                        ws.close();
                        result.innerHTML = '🔄 Waiting for connection attempt from MUCO Manager...';
                        result.style.color = '#666';
                        isTestingConnection = false;
                    }
                }, 3000);
                
            } catch (error) {
                result.innerHTML = '🔄 Waiting for connection attempt from MUCO Manager...';
                result.style.color = '#666';
                isTestingConnection = false;
            }
        }
        
        // Continuously monitor for successful connections by testing periodically
        // This simulates detecting when MUCO Manager attempts to connect
        window.addEventListener('load', function() {
            // Test every 2 seconds to detect when certificate becomes trusted
            setInterval(() => {
                testWebSocket();
            }, 2000);
        });
    </script>
</body>
</html>
    "#;
    
    Ok(warp::reply::html(html))
}

