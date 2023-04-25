using System.Net.Sockets;

public class Message
{
    public static void SendMessage(NetworkStream stream, byte[] sendData)
    {
        var length_bytes = Serialize.SerializeInt(sendData.Length);
        stream.Write(length_bytes, 0, 4);
        stream.Write(sendData, 0, sendData.Length);
    }

    public static void ReceiveMessage(NetworkStream stream, ref byte[] buffer)
    {
        ReceiveNBytes(4, stream, ref buffer);
        var length = Serialize.DeserializeInt(buffer);
        ReceiveNBytes(length, stream, ref buffer);
    }

    public static void ReceiveNBytes(int expected_length, NetworkStream stream, ref byte[] buffer)
    {
        buffer = new byte[expected_length];
        int read_so_far = 0;
        while (read_so_far != expected_length)
        {
            var recv = stream.Read(buffer, read_so_far, buffer.Length - read_so_far);
            read_so_far += recv;
        }
    }
}
