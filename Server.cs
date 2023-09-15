using System.Net;
using System.Net.Sockets;
using System.Text;
using ExtensionMethods;

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
    TcpListener listener = new TcpListener(System.Net.IPAddress.Any, 1302);
    TcpClient? newTcpClient = null;
    List<Task> allTasks = new List<Task>();

    public async Task Run()
    {
        listener.Start();
        Console.WriteLine("Server Started\n");

        Console.WriteLine("Address List:");
        string hostName = Dns.GetHostName();
        var host = Dns.GetHostEntry(hostName);
        var addresList = host.AddressList;
        foreach(var ip in addresList)
        {
            Console.WriteLine("" + ip);
        }
        Console.WriteLine("\n");

        allTasks.Add(AcceptNewClient());

        while(true)
        {
            var task = await Task.WhenAny(allTasks);
            var task_index = allTasks.IndexOf(task);
            if(task_index == 0)
            {
                await ProcessNewClient();
            }
            else
            {
                await ProcessClientReceivedData(task_index);
            }

            await KickDeadClients();
        }
    }

    public async Task DealWithDisconnectedClient(int client_index)
    {
        var client = clients[client_index];
        Console.WriteLine("Client " +  client.id + ": disconnected");
        client.tcp?.Close();
        clients.RemoveAt(client_index);
        allTasks.RemoveAt(client_index + 1);

        var data = new List<byte>();
        Serialize.SerializeInt(data, (int)ServerToClientMessageType.ClientDisconnected);
        Serialize.SerializeInt(data, client.id);
        await BroadcastMessage(data);
    }

    public async Task KickDeadClients()
    {
        for(int client_index = 0; client_index < clients.Count; client_index++)
        {
            var client = clients[client_index];
            if(client.isDead)
                await DealWithDisconnectedClient(client_index);
        }
    }

    public async Task ProcessClientReceivedData(int task_index)
    {
        var client_index = task_index - 1;
        var client = clients[client_index];
        if(client.isDead)
        {
            await DealWithDisconnectedClient(client_index);
        }
        else
        {
            TryPopMessage(client_index);
            allTasks[task_index] = client.ReceiveData();
            await ProcessMessages();
        }
    }

    public async Task ProcessMessages()
    {
        while(messages.Count > 0)
        {
            var (id, message) = messages.Dequeue();
            await ProcessMessage(id, message);
        }
    }

    public void TryPopMessage(int client_index)
    {
        var client = clients[client_index];
        while (client.messageBuffer.Count > 4)
        {
            var length = Serialize.DeserializeInt(client.messageBuffer);
            if (client.messageBuffer.Count < length + 4)
                break;
            
            var message = new byte[length];
            Array.Copy(client.messageBuffer.ToArray(), 4, message, 0, length);

            client.messageBuffer.RemoveRange(0, length + 4);

            messages.Enqueue((client.id, message));
        }
    }

    public async Task AcceptNewClient()
    {
        newTcpClient = await listener.AcceptTcpClientAsync();
    }

    public async Task ProcessNewClient()
    {
        if (newTcpClient == null)
            return;
        
        Client client = new Client(id_counter++, newTcpClient);
        var client_index = clients.Count;
        clients.Add(client);

        {
            var data = new List<byte>();
            Serialize.SerializeInt(data, (int)ServerToClientMessageType.AssignClientId);
            Serialize.SerializeInt(data, client.id);
            await SendMessageClient(data, client);
        }
        
        {
            var data = new List<byte>();
            Serialize.SerializeInt(data, (int)ServerToClientMessageType.ClientConnected);
            Serialize.SerializeInt(data, client.id);
            await BroadcastMessageOther(data, client.id);
        }

        allTasks.Add(clients[client_index].ReceiveData());
        
        Console.WriteLine("Client accepted: " + client.id + " " + client.tcp.Client.RemoteEndPoint);

        allTasks[0] = AcceptNewClient();
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

    async Task ProcessMessage(int clientId, byte[] buffer)
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
                allTasks.RemoveAt(client_index + 1);

                var data = new List<byte>();
                Serialize.SerializeInt(data, (int)ServerToClientMessageType.ClientDisconnected);
                Serialize.SerializeInt(data, clientId);
                await BroadcastMessage(data);
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
                await BroadcastMessage(data);
                break;
            }
            case ClientToServerMessageType.BroadcastBytesAll:
            {
                var data = new List<byte>();
                Serialize.SerializeInt(data, (int)ServerToClientMessageType.BroadcastBytes);
                Serialize.SerializeInt(data, clientId);
                data.AddSlice(restBuffer);
                await BroadcastMessage(data);
                break;
            }
            case ClientToServerMessageType.BroadcastBytesOther:
            {
                var data = new List<byte>();
                Serialize.SerializeInt(data, (int)ServerToClientMessageType.BroadcastBytes);
                Serialize.SerializeInt(data, clientId);
                data.AddSlice(restBuffer);
                await BroadcastMessageOther(data, clientId);
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
                    await SendMessageClient(data, clientInfo);
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
                await SendMessageClient(data, toClient);
                break;
            }
            default:
                Console.WriteLine("Unhandeled message type: " + messageType);
                break;
        }
    }

    async Task BroadcastMessage(List<byte> data)
    {
        var array = data.ToArray();
        foreach(var client in clients)
        {
            await SendMessageClient(array, client);
        }
    }

    async Task BroadcastMessageOther(List<byte> data, int id)
    {
        var array = data.ToArray();
        foreach(var client in clients)
        {
            if (client.id == id)
                continue;
            await SendMessageClient(array, client);
        }
    }

    async Task SendMessageClient(byte[] data, Client client)
    {
        try
        {
            var stream = client.tcp.GetStream();
            await Message.SendMessageAsync(stream, data);
        }
        catch
        {
            Console.WriteLine("Could not send message to client: " + client.id);
            client.isDead = true;
        }
    }

    async Task SendMessageClient(List<byte> data, Client client)
    {
        try
        {
            var stream = client.tcp.GetStream();
            await Message.SendMessageAsync(stream, data);
        }
        catch
        {
            Console.WriteLine("Could not send message to client: " + client.id);
            client.isDead = true;
        }
    }
}
