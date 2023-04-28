using System.Diagnostics.CodeAnalysis;
using System.Net.Sockets;
using System.Runtime.Serialization;
using System.Runtime.Serialization.Formatters.Binary;
using System.Text;

class Client
{
    public int id;
    public TcpClient tcp;
    public List<byte> buffer;

    public Client(int id, [DisallowNull]TcpClient tcpClient)
    {
        this.id = id;
        this.tcp = tcpClient;
        this.buffer = new List<byte>();
    }
}

enum ClientToServerMessageType
{
    Disconnect,
    BroadcastChatMessage,
    BroadcastBytesAll,
    BroadcastBytesOther,
    StoreData,
    RetrieveData,
}

enum ServerToClientMessageType
{
    BroadcastChatMessage,
    BroadcastBytes,
    ClientDisconnected,
    AssignClientId,
    Data,
}

class Server
{
    int id_counter = 0;

    Queue<(int, byte[])> messages = new Queue<(int, byte[])>();
    List<Client> clients = new List<Client>();
    Dictionary<string, byte[]> dataStore = new Dictionary<string, byte[]>();

    public void Run()
    {
        Console.WriteLine("Starting Server");

        TcpListener listener = new TcpListener(System.Net.IPAddress.Any, 1302);
        listener.Start();

        var generalBuffer = new byte[1024];

        while(true)
        {
            while (listener.Pending())
            {
                TcpClient tcpClient = listener.AcceptTcpClient();
                Client client = new Client(id_counter++, tcpClient);
                clients.Add(client);

                var data = new List<byte>();
                data.AddRange(Serialize.SerializeInt((int)ServerToClientMessageType.AssignClientId));
                data.AddRange(Serialize.SerializeInt(client.id));
                BroadcastMessage(data.ToArray());
                var stream = tcpClient.GetStream();
                Message.SendMessage(stream, data.ToArray());

                Console.WriteLine("Client accepted: " + client.id);
            }
            
            for (int client_index = 0; client_index < clients.Count;)
            {
                var client = clients[client_index];
                
                try
                {
                    NetworkStream stream = client.tcp.GetStream();
                    while (stream.DataAvailable)
                    {
                        var recv = stream.Read(generalBuffer, 0, generalBuffer.Length);
                        for (int i = 0; i < recv; i++)
                        {
                            client.buffer.Add(generalBuffer[i]);
                        }
                    }
                    client_index++;
                }
                catch (Exception)
                {
                    Console.WriteLine("Client " +  client.id + ": disconnected");
                    client.tcp?.Close();
                    clients.RemoveAt(client_index);

                    var data = new List<byte>();
                    data.AddRange(Serialize.SerializeInt((int)ServerToClientMessageType.ClientDisconnected));
                    data.AddRange(Serialize.SerializeInt(client.id));
                    BroadcastMessage(data.ToArray());
                }
            }

            for (int client_index = 0; client_index < clients.Count; client_index++)
            {
                var client = clients[client_index];
                while (client.buffer.Count > 4)
                {
                    var length = Serialize.DeserializeInt(client.buffer.ToArray());
                    if (client.buffer.Count < length + 4)
                        break;
                    
                    var message = new byte[length];
                    Array.Copy(client.buffer.ToArray(), 4, message, 0, length);

                    client.buffer.RemoveRange(0, length + 4);

                    messages.Enqueue((client.id, message));
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
                clientInfo.tcp?.Close();
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
            case ClientToServerMessageType.StoreData:
            {
                var stringLength = Serialize.DeserializeInt(restBuffer);
                var stringBytes = new byte[stringLength];
                var dataBytes = new byte[restBuffer.Length - 4 - stringLength];
                Array.Copy(restBuffer, 4, stringBytes, 0, stringLength);
                Array.Copy(restBuffer, 4 + stringLength, dataBytes, 0, restBuffer.Length - 4 - stringLength);
                string label = Encoding.UTF8.GetString(stringBytes, 0, stringBytes.Length);
                dataStore[label] = dataBytes;
                break;
            }
            case ClientToServerMessageType.RetrieveData:
            {
                string label = Encoding.UTF8.GetString(restBuffer, 0, restBuffer.Length);
                if (dataStore.ContainsKey(label))
                {
                    byte[] userDataBytes = dataStore[label];
                    var labelBytes = Encoding.ASCII.GetBytes(label);
                    var data = new List<byte>();
                    data.AddRange(Serialize.SerializeInt((int)ClientToServerMessageType.StoreData));
                    data.AddRange(Serialize.SerializeInt(labelBytes.Length));
                    data.AddRange(labelBytes);
                    data.AddRange(userDataBytes);
                    SendMessageClient(data.ToArray(), clientInfo);
                }
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
            SendMessageClient(data, client);
        }
    }

    void BroadcastMessageOther(byte[] data, int id)
    {
        foreach(var client in clients)
        {
            if (client.id == id)
                continue;
            SendMessageClient(data, client);
        }
    }

    void SendMessageClient(byte[] data, Client client)
    {
        try
        {
            var stream = client.tcp.GetStream();
            Message.SendMessage(stream, data);
        }
        catch
        {
            Console.WriteLine("Could not send message to client: " + client.id);
        }
    }
}
