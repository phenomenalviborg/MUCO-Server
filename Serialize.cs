
public static class Serialize
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

    public static void SerializeInt(List<byte> list, int i)
    {
        list.Add((byte)((i & 0xFF) >> 0));
        list.Add((byte)((i & 0xFFFF) >> 8));
        list.Add((byte)((i & 0xFFFFFF) >> 16));
        list.Add((byte)((i & 0xFFFFFFFF) >> 24));
    }

    public static int DeserializeInt(byte[] buffer)
    {
        int acc = (int)buffer[0];
        acc += (int)buffer[1] << 8;
        acc += (int)buffer[2] << 16;
        acc += (int)buffer[3] << 24;
        return acc;
    }

    public static int DeserializeInt(BufferSlice slice)
    {
        int acc = (int)slice.bytes[slice.begin + 0];
        acc += (int)slice.bytes[slice.begin + 1] << 8;
        acc += (int)slice.bytes[slice.begin + 2] << 16;
        acc += (int)slice.bytes[slice.begin + 3] << 24;
        return acc;
    }
}
