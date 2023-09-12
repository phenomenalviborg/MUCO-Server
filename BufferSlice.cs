using System.Text;

public struct BufferSlice
{
    public byte[] bytes;
    public int begin;
    public int end;

    public BufferSlice(byte[] bytes)
    {
        this.bytes = bytes;
        begin = 0;
        end = bytes.Length;
    }

    public BufferSlice DropBegin(int count)
    {
        return new BufferSlice
        {
            bytes = bytes,
            begin = begin + count,
            end = end,
        };
    }

    public int Length
    {
        get
        {
            return end - begin;
        }
    }

    public (BufferSlice, BufferSlice) SplitAt(int i)
    {
        var a = new BufferSlice
        {
            bytes = bytes,
            begin = begin,
            end = begin + i,
        };
        var b = new BufferSlice
        {
            bytes = bytes,
            begin = begin + i,
            end = end,
        };
        return (a, b);
    }

    public byte[] ToBuffer()
    {
        var buffer = new byte[Length];
        Array.Copy(bytes, begin, buffer, 0, Length);
        return buffer;
    }

    public string ToUTF8()
    {
        return Encoding.UTF8.GetString(bytes, begin, Length);
    }
}

namespace ExtensionMethods
{
    public static class BufferSliceExtensions
    {
        public static void AddSlice(this List<byte> list, BufferSlice slice)
        {
            for(int i = slice.begin; i < slice.end; i++)
                list.Add(slice.bytes[i]);
        }
    }
}
