using System.Net.Sockets;

public class Message
{
    public static void SendMessage(NetworkStream stream, byte[] sendData)
    {
        var length_bytes = Serialize.SerializeInt(sendData.Length);
        stream.Write(length_bytes, 0, 4);
        stream.Write(sendData, 0, sendData.Length);
    }

    public static void SendMessage(NetworkStream stream, List<byte> sendData)
    {
        var length_bytes = Serialize.SerializeInt(sendData.Count);
        stream.Write(length_bytes, 0, 4);
        stream.Write(sendData.ToArray(), 0, sendData.Count);
    }
}
