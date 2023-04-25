
public class Serialize
{
    public static byte[] SerializeInt(int i)
    {
        byte[] bytes = new byte[4];
        bytes[0] = (byte)((i & 0xFF) >> 0);
        bytes[1] = (byte)((i & 0xFFFF) >> 8);
        bytes[2] = (byte)((i & 0xFFFFFF) >> 16);
        bytes[3] = (byte)((i & 0xFFFFFFFF) >> 24);
        return bytes;
    }

    public static int DeserializeInt(byte[] buffer)
    {
        int acc = (int)buffer[0];
        acc += (int)buffer[1] << 8;
        acc += (int)buffer[2] << 16;
        acc += (int)buffer[3] << 24;
        return acc;
    }
}
