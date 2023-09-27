using System.Net.Sockets;

public class Message
{
    public static async Task SendMessageAsync(NetworkStream stream, byte[] sendData)
    {
        var length_bytes = Serialize.SerializeInt(sendData.Length);
        await stream.WriteAsync(length_bytes, 0, 4);
        await stream.WriteAsync(sendData, 0, sendData.Length);
        await stream.FlushAsync();
    }

    public static async Task SendMessageAsync(NetworkStream stream, List<byte> sendData)
    {
        var length_bytes = Serialize.SerializeInt(sendData.Count);
        await stream.WriteAsync(length_bytes, 0, 4);
        await stream.WriteAsync(sendData.ToArray(), 0, sendData.Count);
        await stream.FlushAsync();
    }
}
