using System.Diagnostics.CodeAnalysis;
using System.Net.Sockets;
using System.Text;
using ExtensionMethods;

class Client
{
    public int id;
    public TcpClient tcp;
    public List<byte> buffer;

    public Client(int id, [DisallowNull]TcpClient tcpClient)
    {
        this.id = id;
        tcp = tcpClient;
        buffer = new List<byte>();
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
    BinaryMessageTo,
}

enum ServerToClientMessageType
{
    AssignClientId,
    ClientConnected,
    ClientDisconnected,
    BroadcastChatMessage,
    BroadcastBytes,
    Data,
    BinaryMessageFrom,
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

        Console.WriteLine("Server Started");

        var generalBuffer = new byte[1024];

        while(true)
        {
            bool didSomething = false;
            while (listener.Pending())
            {
                TcpClient tcpClient = listener.AcceptTcpClient();
                Client client = new Client(id_counter++, tcpClient);
                clients.Add(client);

                {
                    var data = new List<byte>();
                    Serialize.SerializeInt(data, (int)ServerToClientMessageType.AssignClientId);
                    Serialize.SerializeInt(data, client.id);
                    SendMessageClient(data.ToArray(), client);
                }
                
                {
                    var data = new List<byte>();
                    Serialize.SerializeInt(data, (int)ServerToClientMessageType.ClientConnected);
                    Serialize.SerializeInt(data, client.id);
                    BroadcastMessageOther(data.ToArray(), client.id);
                }
                
                Console.WriteLine("Client accepted: " + client.id);

                didSomething = true;
            }
            
            for (int client_index = 0; client_index < clients.Count;)
            {
                var client = clients[client_index];
                
                try
                {
                    NetworkStream stream = client.tcp.GetStream();
                    while (stream.DataAvailable)
                    {
                        didSomething = true;
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
                    Serialize.SerializeInt(data, (int)ServerToClientMessageType.ClientDisconnected);
                    Serialize.SerializeInt(data, client.id);
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

            if(!didSomething)
            {
                Thread.Sleep(1);
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
        var restBuffer = new BufferSlice(buffer).DropBegin(4);

        switch (messageType)
        {
            case ClientToServerMessageType.Disconnect:
            {
                Console.WriteLine("Client " +  clientId + ": disconnected");
                clientInfo.tcp?.Close();
                clients.RemoveAt(client_index);

                var data = new List<byte>();
                Serialize.SerializeInt(data, (int)ServerToClientMessageType.ClientDisconnected);
                Serialize.SerializeInt(data, clientId);
                BroadcastMessage(data.ToArray());
                break;
            }
            case ClientToServerMessageType.BroadcastChatMessage:
            {
                var chatMessage = restBuffer.ToUTF8();
                Console.WriteLine("broadcast chat message " + clientInfo.id + ": " + chatMessage);

                var data = new List<byte>();
                Serialize.SerializeInt(data, (int)ServerToClientMessageType.BroadcastChatMessage);
                Serialize.SerializeInt(data, clientId);
                data.AddRange(Encoding.ASCII.GetBytes(chatMessage));
                BroadcastMessage(data.ToArray());
                break;
            }
            case ClientToServerMessageType.BroadcastBytesAll:
            {
                var data = new List<byte>();
                Serialize.SerializeInt(data, (int)ServerToClientMessageType.BroadcastBytes);
                Serialize.SerializeInt(data, clientId);
                data.AddSlice(restBuffer);
                BroadcastMessage(data.ToArray());
                break;
            }
            case ClientToServerMessageType.BroadcastBytesOther:
            {
                var data = new List<byte>();
                Serialize.SerializeInt(data, (int)ServerToClientMessageType.BroadcastBytes);
                Serialize.SerializeInt(data, clientId);
                data.AddSlice(restBuffer);
                BroadcastMessageOther(data.ToArray(), clientId);
                break;
            }
            case ClientToServerMessageType.StoreData:
            {
                var stringLength = Serialize.DeserializeInt(restBuffer);
                restBuffer = restBuffer.DropBegin(4);
                var (stringSlice, dataSlice) = restBuffer.SplitAt(stringLength);
                string label = stringSlice.ToUTF8();
                var dataBytes = dataSlice.ToBuffer();
                dataStore[label] = dataBytes;

                break;
            }
            case ClientToServerMessageType.RetrieveData:
            {
                var label = restBuffer.ToUTF8();
                if (dataStore.ContainsKey(label))
                {
                    byte[] userDataBytes = dataStore[label];
                    var labelBytes = Encoding.ASCII.GetBytes(label);
                    var data = new List<byte>();
                    Serialize.SerializeInt(data, (int)ServerToClientMessageType.Data);
                    Serialize.SerializeInt(data, labelBytes.Length);
                    data.AddRange(labelBytes);
                    data.AddRange(userDataBytes);
                    SendMessageClient(data.ToArray(), clientInfo);
                }
                break;
            }
            case ClientToServerMessageType.BinaryMessageTo:
            {
                var toClientId = Serialize.DeserializeInt(restBuffer);
                restBuffer = restBuffer.DropBegin(4);
                var toClientIndex = GetClientIndex(toClientId);
                var toClient = clients[toClientIndex];
                var data = new List<byte>();
                Serialize.SerializeInt(data, (int)ServerToClientMessageType.BroadcastBytes);
                Serialize.SerializeInt(data, clientId);
                data.AddSlice(restBuffer);
                SendMessageClient(data, toClient);
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

    void SendMessageClient(List<byte> data, Client client)
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
