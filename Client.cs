using System.Diagnostics.CodeAnalysis;
using System.Net.Sockets;

public class Client
{
    public int id;
    public TcpClient tcp;
    public byte[] networkBuffer = new byte[1024];
    public List<byte> messageBuffer;
    public bool isDead = false;

    public Client(int id, [DisallowNull]TcpClient tcpClient)
    {
        this.id = id;
        tcp = tcpClient;
        messageBuffer = new List<byte>();
    }

    public async Task ReceiveData()
    {      
        try
        {
            NetworkStream stream = tcp.GetStream();
            
            var recv = await stream.ReadAsync(networkBuffer, 0, networkBuffer.Length);

            for (int i = 0; i < recv; i++)
            {
                messageBuffer.Add(networkBuffer[i]);
            }
        }
        catch (Exception)
        {
            isDead = true;
        }
    }
}
