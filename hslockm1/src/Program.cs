using System;
using System.IO;
using System.Globalization;
using System.Net;
using System.Runtime.InteropServices;
using System.Text;
using System.Web.Script.Serialization;

internal static class Program
{
    private static readonly JavaScriptSerializer Json = new JavaScriptSerializer();
    private static string ApiToken = "default_secret";
    private static string SectorPath;
    private static int DriverInitializationStatus = -1;

    [StructLayout(LayoutKind.Sequential)]
    private struct V2CardInfo
    {
        public int HotelId, Type, BuildingId, FloorId, RoomId, SubRoomId;
        public int NowSeqId, NowYear, NowMonth, NowDay, NowHour, NowMinute;
        public int EndYear, EndMonth, EndDay, EndHour, EndMinute;
        public int Ext0, Ext1, Ext2, Ext3, Ext4, Ext5, Ext6, Ext7;
        public int Ext8, Ext9, Ext10, Ext11, Ext12, Ext13, Ext14, Ext15;
        public int ReportLost;
    }

    [DllImport("libDriverWrapper_M1.dll", CallingConvention = CallingConvention.Winapi, CharSet = CharSet.Ansi, ExactSpelling = true)]
    private static extern int V2WriteCardEx(string portName, int sector, ref V2CardInfo card);

    [DllImport("libDriverWrapper_M1.dll", CallingConvention = CallingConvention.Winapi, CharSet = CharSet.Ansi, ExactSpelling = true)]
    private static extern int V2ReadCardEx(string portName, int sector, ref V2CardInfo card);

    [DllImport("libDriverWrapper_M1.dll", CallingConvention = CallingConvention.Winapi, CharSet = CharSet.Ansi, ExactSpelling = true)]
    private static extern int V2ClearCardEx(string portName, int sector);

    [DllImport("libDriver_M1-0.dll", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    private static extern int BuildDriver();

    [DllImport("libDriver_M1-0.dll", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    private static extern int DestroyDriver();

    private sealed class CardRequest
    {
        public int hotel_id { get; set; }
        public string lock_no { get; set; }
        public string checkout_time { get; set; }
        public string port_name { get; set; }
    }

    public static void Main()
    {
        string applicationDirectory = AppDomain.CurrentDomain.BaseDirectory;
        Directory.SetCurrentDirectory(applicationDirectory);
        SectorPath = Path.Combine(applicationDirectory, "sector.cfg");
        string tokenPath = Path.Combine(applicationDirectory, "auth");
        if (File.Exists(tokenPath)) ApiToken = File.ReadAllText(tokenPath).Trim();

        DriverInitializationStatus = BuildDriver();
        Console.WriteLine("HS Lock native driver initialization status: " + DriverInitializationStatus);
        AppDomain.CurrentDomain.ProcessExit += delegate
        {
            if (DriverInitializationStatus == 0) DestroyDriver();
        };

        HttpListener listener = new HttpListener();
        listener.Prefixes.Add("http://127.0.0.1:9001/");
        listener.Start();
        Console.WriteLine("Venus HS Lock M1 Encoder API running on http://127.0.0.1:9001");

        while (true) Handle(listener.GetContext());
    }

    private static void Handle(HttpListenerContext context)
    {
        try
        {
            AddCors(context.Response);
            if (context.Request.HttpMethod == "OPTIONS") { context.Response.StatusCode = 204; context.Response.Close(); return; }
            if (context.Request.Headers["Authorization"] != "Bearer " + ApiToken)
            {
                context.Response.StatusCode = 401;
                WriteJson(context.Response, new { status = "error", message = "Invalid bearer token" });
                return;
            }

            string path = context.Request.Url.AbsolutePath.TrimEnd('/');
            if (context.Request.HttpMethod == "GET" && path == "/test")
            {
                WriteJson(context.Response, new {
                    status = DriverInitializationStatus == 0 ? "success" : "error",
                    driver = "HS Lock SDK 2022 M1 Standalone",
                    initialization_status = DriverInitializationStatus,
                    sector = SavedSector()
                });
            }
            else if (DriverInitializationStatus != 0)
            {
                WriteJson(context.Response, new { status = "error", message = "HS Lock native driver initialization failed", initialization_status = DriverInitializationStatus });
            }
            else if (context.Request.HttpMethod == "GET" && path == "/card/read-guest") ReadCard(context);
            else if (context.Request.HttpMethod == "POST" && path == "/card/make-guest") WriteCard(context);
            else if (context.Request.HttpMethod == "POST" && path == "/card/cancel") ClearCard(context);
            else { context.Response.StatusCode = 404; WriteJson(context.Response, new { status = "error", message = "Endpoint not found" }); }
        }
        catch (Exception exception)
        {
            context.Response.StatusCode = 500;
            WriteJson(context.Response, new { status = "error", message = exception.Message });
        }
    }

    private static void ReadCard(HttpListenerContext context)
    {
        string port = NormalizePort(context.Request.QueryString["port_name"]);
        V2CardInfo card = new V2CardInfo();
        int sector;
        int code;
        if (!TryReadCard(port, out card, out sector, out code))
        {
            WriteJson(context.Response, new {
                status = "error",
                message = "HS Lock read failed: no valid HS M1 card data was found in sectors 1-15. Close the original lock software and use an existing encoded guest card.",
                sdk_code = code,
                port_name = port,
                sectors_tested = "1-15"
            });
            return;
        }
        SaveSector(sector);

        WriteJson(context.Response, new {
            status = "success",
            message = "Card read successfully",
            card_info = new {
                hotel_id = card.HotelId,
                card_type = CardType(card.Type),
                lock_no = LockNumber(card),
                checkin_time = DateValue(card.NowYear, card.NowMonth, card.NowDay, card.NowHour, card.NowMinute),
                checkout_time = DateValue(card.EndYear, card.EndMonth, card.EndDay, card.EndHour, card.EndMinute),
                port_name = port,
                sector = sector
            }
        });
    }

    private static void WriteCard(HttpListenerContext context)
    {
        CardRequest request = Body<CardRequest>(context.Request);
        string error = ValidateWrite(request);
        if (error != null) { WriteJson(context.Response, new { status = "error", message = error }); return; }

        DateTime checkout;
        DateTime.TryParseExact(request.checkout_time, "yyyy-MM-dd HH:mm:ss", CultureInfo.InvariantCulture, DateTimeStyles.None, out checkout);
        DateTime now = DateTime.Now;
        V2CardInfo card = new V2CardInfo {
            HotelId = request.hotel_id, Type = 1,
            BuildingId = IntPart(request.lock_no, 0), FloorId = IntPart(request.lock_no, 2), RoomId = IntPart(request.lock_no, 4), SubRoomId = 15,
            NowSeqId = now.Second, NowYear = now.Year, NowMonth = now.Month, NowDay = now.Day, NowHour = now.Hour, NowMinute = now.Minute,
            EndYear = checkout.Year, EndMonth = checkout.Month, EndDay = checkout.Day, EndHour = checkout.Hour, EndMinute = checkout.Minute,
            Ext0 = 255, Ext1 = 255, Ext2 = 255, Ext3 = 15, Ext4 = 255, Ext5 = 255, Ext6 = 255, Ext7 = 15
        };
        string port = NormalizePort(request.port_name);
        int sector = SavedSector();
        int code = V2WriteCardEx(port, sector, ref card);
        if (code != 0) { SdkError(context.Response, "write", code, port, sector); return; }

        V2CardInfo verified = new V2CardInfo();
        int verifyCode = V2ReadCardEx(port, sector, ref verified);
        if (verifyCode != 0 || verified.HotelId != request.hotel_id || LockNumber(verified) != request.lock_no)
        {
            WriteJson(context.Response, new { status = "error", message = "Card was written but verification failed", sdk_code = verifyCode });
            return;
        }
        WriteJson(context.Response, new { status = "success", message = "HS Lock guest card written and verified", lock_no = request.lock_no, sector = sector });
    }

    private static void ClearCard(HttpListenerContext context)
    {
        CardRequest request = Body<CardRequest>(context.Request) ?? new CardRequest();
        string port = NormalizePort(request.port_name);
        int sector = SavedSector();
        int code = V2ClearCardEx(port, sector);
        if (code != 0) { SdkError(context.Response, "clear", code, port, sector); return; }
        WriteJson(context.Response, new { status = "success", message = "HS Lock card cleared", sector = sector });
    }

    private static string ValidateWrite(CardRequest request)
    {
        if (request == null || request.hotel_id <= 0) return "HS Lock Hotel/System ID is required";
        if (request.lock_no == null || request.lock_no.Length != 6) return "HS Lock number must be six digits: building, floor, and room";
        foreach (char value in request.lock_no) if (!char.IsDigit(value)) return "HS Lock number must contain digits only";
        DateTime checkout;
        if (!DateTime.TryParseExact(request.checkout_time, "yyyy-MM-dd HH:mm:ss", CultureInfo.InvariantCulture, DateTimeStyles.None, out checkout) || checkout <= DateTime.Now) return "A future checkout time is required";
        return null;
    }

    private static int IntPart(string value, int start) { return int.Parse(value.Substring(start, 2)); }
    private static string LockNumber(V2CardInfo card) { return card.BuildingId.ToString("00") + card.FloorId.ToString("00") + card.RoomId.ToString("00"); }
    private static string DateValue(int y, int m, int d, int h, int minute) { try { return new DateTime(y, m, d, h, minute, 0).ToString("yyyy-MM-dd HH:mm:ss"); } catch { return ""; } }
    private static string NormalizePort(string value) { return string.IsNullOrWhiteSpace(value) ? "COM1" : value.Trim().ToUpperInvariant(); }
    private static string CardType(int type) { return type == 0 ? "blank" : type == 1 ? "guest" : type.ToString(); }

    private static bool TryReadCard(string port, out V2CardInfo card, out int detectedSector, out int lastCode)
    {
        int preferredSector = SavedSector();
        card = new V2CardInfo();
        lastCode = V2ReadCardEx(port, preferredSector, ref card);
        if (lastCode == 0) { detectedSector = preferredSector; return true; }

        for (int sector = 1; sector <= 15; sector++)
        {
            if (sector == preferredSector) continue;
            card = new V2CardInfo();
            lastCode = V2ReadCardEx(port, sector, ref card);
            if (lastCode == 0) { detectedSector = sector; return true; }
        }

        detectedSector = preferredSector;
        return false;
    }

    private static int SavedSector()
    {
        int sector;
        return File.Exists(SectorPath)
            && int.TryParse(File.ReadAllText(SectorPath).Trim(), out sector)
            && sector >= 1 && sector <= 15 ? sector : 1;
    }

    private static void SaveSector(int sector)
    {
        File.WriteAllText(SectorPath, sector.ToString(CultureInfo.InvariantCulture));
    }

    private static void SdkError(HttpListenerResponse response, string operation, int code, string port, int sector)
    {
        string detail = code == -1 ? "No response from the card or encoder" : code == 1 ? "Invalid parameters" : code == 2 ? "Unable to open the COM port" : code == 3 ? "Unable to write to the encoder" : code == 4 ? "Unable to read from the encoder or card" : "Unknown SDK error";
        WriteJson(response, new { status = "error", message = "HS Lock " + operation + " failed: " + detail, sdk_code = code, port_name = port, sector = sector });
    }

    private static T Body<T>(HttpListenerRequest request) where T : class
    {
        using (StreamReader reader = new StreamReader(request.InputStream, request.ContentEncoding ?? Encoding.UTF8))
        {
            string content = reader.ReadToEnd();
            return string.IsNullOrWhiteSpace(content) ? null : Json.Deserialize<T>(content);
        }
    }

    private static void AddCors(HttpListenerResponse response)
    {
        response.Headers["Access-Control-Allow-Origin"] = "*";
        response.Headers["Access-Control-Allow-Methods"] = "GET, POST, OPTIONS";
        response.Headers["Access-Control-Allow-Headers"] = "Authorization, Content-Type";
    }

    private static void WriteJson(HttpListenerResponse response, object payload)
    {
        byte[] content = Encoding.UTF8.GetBytes(Json.Serialize(payload));
        response.ContentType = "application/json; charset=utf-8";
        response.ContentLength64 = content.Length;
        response.OutputStream.Write(content, 0, content.Length);
        response.OutputStream.Close();
    }
}
