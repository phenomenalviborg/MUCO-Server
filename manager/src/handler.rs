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
        button {
            background: #0277bd;
            color: white;
            border: none;
            padding: 0.75rem 1.5rem;
            border-radius: 4px;
            cursor: pointer;
            font-size: 1rem;
            display: block;
            margin: 1rem auto;
        }
        button:hover {
            background: #01579b;
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
            ✅ Certificate Trust Successful!
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
        
        <h3>Next Steps:</h3>
        <ol>
            <li>Return to your MUCO Manager frontend</li>
            <li>Enter this server's IP address in the "Servers" section</li>
            <li>The WebSocket connection should now work without certificate errors</li>
        </ol>
        
        <button onclick="testWebSocket()">Test WebSocket Connection</button>
        
        <div id="test-result" style="margin-top: 1rem; text-align: center;"></div>
        
        <div class="footer">
            MUCO Server Dynamic SSL Certificate<br>
            Generated automatically for secure connections
        </div>
    </div>

    <script>
        function testWebSocket() {
            const result = document.getElementById('test-result');
            result.innerHTML = '🔄 Testing WebSocket connection...';
            
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const wsUrl = `${protocol}//${window.location.host}/ws`;
            
            try {
                const ws = new WebSocket(wsUrl);
                
                ws.onopen = function() {
                    result.innerHTML = '✅ WebSocket connection successful!';
                    result.style.color = '#16a34a';
                    ws.close();
                };
                
                ws.onerror = function() {
                    result.innerHTML = '❌ WebSocket connection failed';
                    result.style.color = '#dc2626';
                };
                
                setTimeout(() => {
                    if (ws.readyState === WebSocket.CONNECTING) {
                        ws.close();
                        result.innerHTML = '⏱️ Connection timeout - check server status';
                        result.style.color = '#ea580c';
                    }
                }, 5000);
                
            } catch (error) {
                result.innerHTML = '❌ WebSocket test failed: ' + error.message;
                result.style.color = '#dc2626';
            }
        }
    </script>
</body>
</html>
    "#;
    
    Ok(warp::reply::html(html))
}

