using System.Diagnostics.CodeAnalysis;
using System.Net.Sockets;
using System.Text;

class ClientInfo
{
    public int id;
    public TcpClient tcpClient;

    public ClientInfo(int id, [DisallowNull]TcpClient tcpClient)
    {
        this.id = id;
        this.tcpClient = tcpClient;
    }
}

class Server
{
    
    enum ClientToServerMessageType
    {
        Disconnect,
        BroadcastChatMessage,
        BroadcastBytesAll,
        BroadcastBytesOther,
    }

    enum ServerToClientMessageType
    {
        BroadcastChatMessage,
        BroadcastBytes,
        ClientDisconnected,
    }

    int id_counter = 0;

    List<ClientInfo> clients = new List<ClientInfo>();
    Queue<(int, byte[])> messages = new Queue<(int, byte[])>();

    public void Run()
    {
        Console.WriteLine("Starting Server");

        TcpListener listener = new TcpListener(System.Net.IPAddress.Any, 1302);
        listener.Start();

        while(true)
        {
            while (listener.Pending())
            {
                TcpClient tcpClient = listener.AcceptTcpClient();
                ClientInfo client = new ClientInfo(id_counter++, tcpClient);
                clients.Add(client);
                Console.WriteLine("Client accepted: " + client.id);
            }
            
            for (int client_index = 0; client_index < clients.Count;)
            {
                var client = clients[client_index];
                
                try
                {
                    NetworkStream stream = client.tcpClient.GetStream();
                    if (stream.DataAvailable)
                    {
                        byte[] buffer = new byte[0];
                        Message.ReceiveMessage(stream, ref buffer);
                        messages.Enqueue((client.id, buffer));
                        client_index++;
                    }
                    else
                    {
                        client_index++;
                    }
                }
                catch (Exception)
                {
                    Console.WriteLine("Client " +  client.id + ": disconnected");
                    client.tcpClient?.Close();
                    clients.RemoveAt(client_index);

                    var data = new List<byte>();
                    data.AddRange(Serialize.SerializeInt((int)ServerToClientMessageType.ClientDisconnected));
                    data.AddRange(Serialize.SerializeInt(client.id));
                    BroadcastMessage(data.ToArray());
                }
            }

            while(messages.Count > 0)
            {
                var (id, message) = messages.Dequeue();
                ProcessMessage(id, ref message);
            }
        }
    }

    int GetClientIndex(int id)
    {
        for (int client_index = 0; client_index < clients.Count; client_index++)
        {
            var client = clients[client_index];
            if (client.id == id)
                return client_index;
        }
        return -1;
    }

    void ProcessMessage(int clientId, ref byte[] buffer)
    {
        var client_index = GetClientIndex(clientId);
        var clientInfo = clients[client_index];

        var messageType = (ClientToServerMessageType)Serialize.DeserializeInt(buffer);
        var restBuffer = new byte[buffer.Length - 4];
        Array.Copy(buffer, 4, restBuffer, 0, buffer.Length - 4);

        switch (messageType)
        {
            case ClientToServerMessageType.Disconnect:
            {
                Console.WriteLine("Client " +  clientId + ": disconnected");
                clientInfo.tcpClient?.Close();
                clients.RemoveAt(client_index);

                var data = new List<byte>();
                data.AddRange(Serialize.SerializeInt((int)ServerToClientMessageType.ClientDisconnected));
                data.AddRange(Serialize.SerializeInt(clientInfo.id));
                BroadcastMessage(data.ToArray());
                break;
            }
            case ClientToServerMessageType.BroadcastChatMessage:
            {
                string chatMessage = Encoding.UTF8.GetString(restBuffer, 0, restBuffer.Length);
                Console.WriteLine("broadcast chat message " + clientInfo.id + ": " + chatMessage);

                var data = new List<byte>();
                data.AddRange(Serialize.SerializeInt((int)ServerToClientMessageType.BroadcastChatMessage));
                data.AddRange(Serialize.SerializeInt(clientInfo.id));
                data.AddRange(Encoding.ASCII.GetBytes(chatMessage));
                BroadcastMessage(data.ToArray());
                break;
            }
            case ClientToServerMessageType.BroadcastBytesAll:
            {
                var data = new List<byte>();
                data.AddRange(Serialize.SerializeInt((int)ServerToClientMessageType.BroadcastBytes));
                data.AddRange(Serialize.SerializeInt(clientInfo.id));
                data.AddRange(restBuffer);
                BroadcastMessage(data.ToArray());
                break;
            }
            case ClientToServerMessageType.BroadcastBytesOther:
            {
                var data = new List<byte>();
                data.AddRange(Serialize.SerializeInt((int)ServerToClientMessageType.BroadcastBytes));
                data.AddRange(Serialize.SerializeInt(clientInfo.id));
                data.AddRange(restBuffer);
                BroadcastMessageOther(data.ToArray(), clientId);
                break;
            }
            default:
                Console.WriteLine("Unhandeled message type: " + messageType);
                break;
        }
    }

    void BroadcastMessage(byte[] data)
    {
        foreach(var client in clients)
        {
            try
            {
                var stream = client.tcpClient.GetStream();
                Message.SendMessage(stream, data);
            }
            catch
            {
                Console.WriteLine("Could not send message to client: " + client.id);
            }
        }
    }

    void BroadcastMessageOther(byte[] data, int id)
    {
        foreach(var client in clients)
        {
            if (client.id == id)
                continue;
            try
            {
                var stream = client.tcpClient.GetStream();
                Message.SendMessage(stream, data);
            }
            catch
            {
                Console.WriteLine("Could not send message to client: " + client.id);
            }
        }
    }
}
