use crate::{ws, Result, context::MucoContextRef};
use warp::{http::StatusCode, Reply};

pub async fn ws_handler(ws: warp::ws::Ws, context_ref: MucoContextRef) -> Result<impl Reply> {
    Ok(ws.on_upgrade(move |socket| ws::frontend_connection_process(socket, context_ref)))
}

pub async fn health_handler() -> Result<impl Reply> {
    Ok(StatusCode::OK)
}

pub async fn trust_handler() -> Result<impl Reply> {
    let html = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Certificate Trusted</title>
    <style>
        body { 
            font-family: system-ui, sans-serif; 
            text-align: center; 
            margin: 50px; 
            background: #f5f5f5; 
        }
        .container { 
            background: white; 
            padding: 40px; 
            border-radius: 8px; 
            display: inline-block; 
        }
        .title { 
            font-size: 24px; 
            color: #16a34a; 
            margin-bottom: 20px; 
        }
        .countdown { 
            font-size: 18px; 
            color: #666; 
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="title">✅ Certificate Trusted</div>
        <div class="countdown" id="status">Testing connection...</div>
    </div>

    <script>
        function testAndClose() {
            const status = document.getElementById('status');
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const wsUrl = `${protocol}//${window.location.host}/ws`;
            
            try {
                const ws = new WebSocket(wsUrl);
                ws.onopen = function() {
                    console.log('✅ WebSocket connection successful!');
                    status.textContent = 'Connected! Notifying parent...';
                    ws.close();
                    
                    // Send message to parent window
                    if (window.opener) {
                        console.log('Sending trust-complete message to parent');
                        window.opener.postMessage({ type: 'trust-complete' }, '*');
                        status.textContent = 'Connected! Closing in 2 seconds...';
                        setTimeout(() => {
                            console.log('Closing trust tab');
                            window.close();
                        }, 2000);
                    } else {
                        console.log('No window.opener found');
                        status.textContent = 'Connected! You can close this tab.';
                    }
                };
            } catch (e) {
                console.log('WebSocket test failed:', e);
            }
        }
        
        testAndClose();
        setInterval(testAndClose, 2000);
    </script>
</body>
</html>"#;
    
    Ok(warp::reply::html(html))
}

