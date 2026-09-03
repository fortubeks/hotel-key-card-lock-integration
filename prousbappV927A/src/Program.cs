using System;
using System.IO;
using System.Net;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using System.Web.Script.Serialization;

internal static class Program
{
    private const int BufferSize = 512;
    private static readonly JavaScriptSerializer Json = new JavaScriptSerializer();
    private static string ApiToken = "default_secret";

    [DllImport("proRFL.dll", CallingConvention = CallingConvention.StdCall)]
    private static extern int initializeUSB(byte usbType);

    [DllImport("proRFL.dll", CallingConvention = CallingConvention.StdCall, CharSet = CharSet.Ansi)]
    private static extern int ReadCard(byte usbType, StringBuilder cardData);

    [DllImport("proRFL.dll", CallingConvention = CallingConvention.StdCall, CharSet = CharSet.Ansi)]
    private static extern int GuestCard(
        byte usbType,
        int hotelId,
        byte cardNo,
        byte dai,
        byte unlockDeadbolt,
        byte publicDoors,
        string beginDate,
        string endDate,
        string lockNo,
        StringBuilder cardData);

    [DllImport("proRFL.dll", CallingConvention = CallingConvention.StdCall, CharSet = CharSet.Ansi)]
    private static extern int CardErase(byte usbType, int hotelId, StringBuilder cardData);

    [DllImport("proRFL.dll", CallingConvention = CallingConvention.StdCall, CharSet = CharSet.Ansi)]
    private static extern int GetCardTypeByCardDataStr(string cardData, StringBuilder cardType);

    [DllImport("proRFL.dll", CallingConvention = CallingConvention.StdCall, CharSet = CharSet.Ansi)]
    private static extern int GetGuestLockNoByCardDataStr(int hotelId, string cardData, StringBuilder lockNo);

    [DllImport("proRFL.dll", CallingConvention = CallingConvention.StdCall, CharSet = CharSet.Ansi)]
    private static extern int GetGuestETimeByCardDataStr(int hotelId, string cardData, StringBuilder checkoutTime);

    [DllImport("proRFL.dll", CallingConvention = CallingConvention.StdCall)]
    private static extern int Buzzer(byte usbType, int duration);

    private sealed class IssueRequest
    {
        public int hotel_id { get; set; }
        public byte card_no { get; set; }
        public byte dai { get; set; }
        public byte unlock_deadbolt { get; set; }
        public string begin_date { get; set; }
        public string end_date { get; set; }
        public string lock_no { get; set; }
    }

    private sealed class HotelRequest
    {
        public int hotel_id { get; set; }
    }

    public static void Main()
    {
        string applicationDirectory = AppDomain.CurrentDomain.BaseDirectory;
        Directory.SetCurrentDirectory(applicationDirectory);

        string tokenPath = Path.Combine(applicationDirectory, "config", "auth");
        if (File.Exists(tokenPath))
        {
            ApiToken = File.ReadAllText(tokenPath).Trim();
        }

        int initializationStatus = initializeUSB(1);
        Console.WriteLine("ProUSB initialization status: " + initializationStatus);

        HttpListener listener = new HttpListener();
        listener.Prefixes.Add("http://127.0.0.1:9001/");
        listener.Start();

        Console.WriteLine("Venus ProUSB 9.27A Encoder API running on http://127.0.0.1:9001");

        while (true)
        {
            HttpListenerContext context = listener.GetContext();
            HandleRequest(context);
        }
    }

    private static void HandleRequest(HttpListenerContext context)
    {
        try
        {
            AddCorsHeaders(context.Response);

            if (context.Request.HttpMethod == "OPTIONS")
            {
                context.Response.StatusCode = 204;
                context.Response.Close();
                return;
            }

            string authorization = context.Request.Headers["Authorization"];
            if (authorization != "Bearer " + ApiToken)
            {
                context.Response.StatusCode = 401;
                WriteJson(context.Response, new { status = "error", message = "Invalid bearer token" });
                return;
            }

            string path = context.Request.Url.AbsolutePath.TrimEnd('/');
            if (context.Request.HttpMethod == "GET" && path == "/card/read-guest")
            {
                ReadGuestCard(context.Request, context.Response);
                return;
            }

            if (context.Request.HttpMethod == "POST" && path == "/card/make-guest")
            {
                IssueGuestCard(context.Request, context.Response);
                return;
            }

            if (context.Request.HttpMethod == "POST" && path == "/card/cancel")
            {
                EraseCard(context.Request, context.Response);
                return;
            }

            context.Response.StatusCode = 404;
            WriteJson(context.Response, new { status = "error", message = "Endpoint not found" });
        }
        catch (Exception exception)
        {
            context.Response.StatusCode = 500;
            WriteJson(context.Response, new { status = "error", message = exception.Message });
        }
    }

    private static void ReadGuestCard(HttpListenerRequest request, HttpListenerResponse response)
    {
        StringBuilder cardBuffer = NewBuffer();
        int readStatus = ReadCard(1, cardBuffer);
        if (readStatus != 0)
        {
            WriteJson(response, new { status = "error", message = "ReadCard failed with SDK code " + readStatus, card_info = (object)null });
            return;
        }

        string rawCardData = cardBuffer.ToString();
        int derivedHotelId = DeriveHotelId(rawCardData);
        int configuredHotelId;
        int decodeHotelId = int.TryParse(request.QueryString["hotel_id"], out configuredHotelId) && configuredHotelId > 0
            ? configuredHotelId
            : derivedHotelId;
        StringBuilder typeBuffer = NewBuffer();
        StringBuilder lockBuffer = NewBuffer();
        StringBuilder checkoutBuffer = NewBuffer();
        int typeStatus = GetCardTypeByCardDataStr(rawCardData, typeBuffer);
        int lockStatus = GetGuestLockNoByCardDataStr(decodeHotelId, rawCardData, lockBuffer);
        int checkoutStatus = GetGuestETimeByCardDataStr(decodeHotelId, rawCardData, checkoutBuffer);

        string message = lockStatus == 0 && checkoutStatus == 0
            ? "Card read and decoded successfully"
            : "Physical card read succeeded, but one or more fields could not be decoded. Check the SDK codes and configured Hotel/Company ID.";

        WriteJson(response, new
        {
            status = "success",
            message = message,
            card_info = new
            {
                card_snr = CardSerial(rawCardData),
                raw_card_data = rawCardData,
                card_type = typeBuffer.ToString(),
                lock_no = lockBuffer.ToString(),
                checkin_time = "N/A",
                checkout_time = checkoutBuffer.ToString(),
                hotel_id = decodeHotelId,
                derived_hotel_id = derivedHotelId,
                hotel_id_source = decodeHotelId == configuredHotelId && configuredHotelId > 0 ? "configured" : "card-derived",
                card_type_status = typeStatus,
                lock_status = lockStatus,
                checkout_status = checkoutStatus
            }
        });
    }

    private static void IssueGuestCard(HttpListenerRequest request, HttpListenerResponse response)
    {
        IssueRequest issue = DeserializeBody<IssueRequest>(request);
        string validationError = ValidateIssueRequest(issue);
        if (validationError != null)
        {
            WriteJson(response, new { status = "error", message = validationError, card_snr = (string)null });
            return;
        }

        StringBuilder existingCardBuffer = NewBuffer();
        int readStatus = ReadCard(1, existingCardBuffer);
        if (readStatus != 0)
        {
            WriteJson(response, new { status = "error", message = "ReadCard failed with SDK code " + readStatus + ". Card was not changed.", card_snr = (string)null });
            return;
        }

        string existingCardData = existingCardBuffer.ToString();
        StringBuilder typeBuffer = NewBuffer();
        int typeStatus = GetCardTypeByCardDataStr(existingCardData, typeBuffer);
        if (typeStatus != 0)
        {
            WriteJson(response, new { status = "error", message = "Card type validation failed with SDK code " + typeStatus + ". Card was not changed.", card_snr = CardSerial(existingCardData) });
            return;
        }

        string cardType = typeBuffer.ToString().ToUpperInvariant();
        if (cardType == "6")
        {
            StringBuilder existingLockBuffer = NewBuffer();
            int hotelValidationStatus = GetGuestLockNoByCardDataStr(issue.hotel_id, existingCardData, existingLockBuffer);
            if (hotelValidationStatus != 0)
            {
                WriteJson(response, new
                {
                    status = "error",
                    message = string.Format(
                        "Existing guest card is not valid for configured hotel {0} (SDK code {1}, derived ID {2}). Card was not changed.",
                        issue.hotel_id,
                        hotelValidationStatus,
                        DeriveHotelId(existingCardData)),
                    card_snr = CardSerial(existingCardData)
                });
                return;
            }
        }
        // TEMPORARILY DISABLED: Some ProUSB 9.27A installations report
        // reusable cards with a type other than guest (6) or blank (F).
        // Do not block those cards before calling the SDK's GuestCard function.
        // else if (cardType != "F")
        // {
        //     WriteJson(response, new
        //     {
        //         status = "error",
        //         message = "Card type " + (cardType.Length == 0 ? "unknown" : cardType) + " cannot be overwritten. Use a blank card or an existing guest card.",
        //         card_snr = CardSerial(existingCardData)
        //     });
        //     return;
        // }

        StringBuilder writtenData = NewBuffer();
        int writeStatus = GuestCard(
            1,
            issue.hotel_id,
            issue.card_no,
            issue.dai,
            issue.unlock_deadbolt,
            0,
            issue.begin_date,
            issue.end_date,
            issue.lock_no,
            writtenData);

        if (writeStatus != 0)
        {
            WriteJson(response, new { status = "error", message = "GuestCard failed with SDK code " + writeStatus, card_snr = CardSerial(existingCardData) });
            return;
        }

        Thread.Sleep(150);
        StringBuilder verificationBuffer = NewBuffer();
        int verificationReadStatus = ReadCard(1, verificationBuffer);
        if (verificationReadStatus != 0)
        {
            WriteJson(response, new { status = "error", message = "Card was written but verification read failed with SDK code " + verificationReadStatus, card_snr = (string)null });
            return;
        }

        string verificationData = verificationBuffer.ToString();
        StringBuilder verifiedLockBuffer = NewBuffer();
        StringBuilder verifiedCheckoutBuffer = NewBuffer();
        int lockStatus = GetGuestLockNoByCardDataStr(issue.hotel_id, verificationData, verifiedLockBuffer);
        int checkoutStatus = GetGuestETimeByCardDataStr(issue.hotel_id, verificationData, verifiedCheckoutBuffer);
        if (lockStatus != 0 || checkoutStatus != 0)
        {
            WriteJson(response, new
            {
                status = "error",
                message = string.Format("Card was written but hotel verification failed (lock SDK code {0}, checkout SDK code {1})", lockStatus, checkoutStatus),
                card_snr = CardSerial(verificationData)
            });
            return;
        }

        string verifiedLock = verifiedLockBuffer.ToString();
        if (verifiedLock != issue.lock_no)
        {
            WriteJson(response, new
            {
                status = "error",
                message = string.Format("Card was written for lock {0}, but ProUSB decoded lock {1}. Check the room lock number.", issue.lock_no, verifiedLock),
                card_snr = CardSerial(verificationData)
            });
            return;
        }

        Buzzer(1, 20);
        WriteJson(response, new
        {
            status = "success",
            message = string.Format("Card verified for hotel {0}, lock {1}, checkout {2}", issue.hotel_id, verifiedLock, verifiedCheckoutBuffer),
            card_snr = CardSerial(verificationData)
        });
    }

    private static void EraseCard(HttpListenerRequest request, HttpListenerResponse response)
    {
        HotelRequest erase = DeserializeBody<HotelRequest>(request);
        if (erase == null || erase.hotel_id <= 0)
        {
            WriteJson(response, new { status = "error", message = "ProUSB Hotel/Company ID is required", card_snr = (string)null });
            return;
        }

        StringBuilder cardBuffer = NewBuffer();
        int readStatus = ReadCard(1, cardBuffer);
        if (readStatus != 0)
        {
            WriteJson(response, new { status = "error", message = "ReadCard failed with SDK code " + readStatus, card_snr = (string)null });
            return;
        }

        string cardData = cardBuffer.ToString();
        StringBuilder lockBuffer = NewBuffer();
        int validationStatus = GetGuestLockNoByCardDataStr(erase.hotel_id, cardData, lockBuffer);
        if (validationStatus != 0)
        {
            WriteJson(response, new { status = "error", message = "Card is not a guest card for the configured hotel. Card was not changed.", card_snr = CardSerial(cardData) });
            return;
        }

        int eraseStatus = CardErase(1, erase.hotel_id, cardBuffer);
        if (eraseStatus != 0)
        {
            WriteJson(response, new { status = "error", message = "CardErase failed with SDK code " + eraseStatus, card_snr = CardSerial(cardData) });
            return;
        }

        Buzzer(1, 20);
        WriteJson(response, new { status = "success", message = "Card erased", card_snr = CardSerial(cardData) });
    }

    private static string ValidateIssueRequest(IssueRequest issue)
    {
        if (issue == null || issue.hotel_id <= 0)
        {
            return "ProUSB Hotel/Company ID is required and must be greater than zero";
        }

        if (!IsTenDigitDate(issue.begin_date) || !IsTenDigitDate(issue.end_date))
        {
            return "Check-in and checkout dates must contain exactly 10 digits in YYMMDDHHMM format";
        }

        if (string.IsNullOrWhiteSpace(issue.lock_no) || issue.lock_no.Length > 8)
        {
            return "ProUSB lock number must contain 1 to 8 characters";
        }

        foreach (char character in issue.lock_no)
        {
            if (!char.IsLetterOrDigit(character) || character > 127)
            {
                return "ProUSB lock number may contain only ASCII letters and digits";
            }
        }

        return null;
    }

    private static bool IsTenDigitDate(string value)
    {
        if (value == null || value.Length != 10)
        {
            return false;
        }

        foreach (char character in value)
        {
            if (character < '0' || character > '9')
            {
                return false;
            }
        }

        return true;
    }

    private static int DeriveHotelId(string cardData)
    {
        if (string.IsNullOrEmpty(cardData) || cardData.Length < 14)
        {
            return 0;
        }

        int low;
        int high;
        // The ProUSB card layout stores dlsCoID in characters 9-14 (1-based):
        // one high byte followed by a 16-bit value whose lower 14 bits are used.
        if (!int.TryParse(cardData.Substring(10, 4), System.Globalization.NumberStyles.HexNumber, null, out low)
            || !int.TryParse(cardData.Substring(8, 2), System.Globalization.NumberStyles.HexNumber, null, out high))
        {
            return 0;
        }

        return (low % 16384) + (high * 65536);
    }

    private static string CardSerial(string cardData)
    {
        return !string.IsNullOrEmpty(cardData) && cardData.Length >= 32
            ? cardData.Substring(24, 8)
            : string.Empty;
    }

    private static StringBuilder NewBuffer()
    {
        return new StringBuilder(BufferSize);
    }

    private static T DeserializeBody<T>(HttpListenerRequest request) where T : class
    {
        using (StreamReader reader = new StreamReader(request.InputStream, request.ContentEncoding ?? Encoding.UTF8))
        {
            string body = reader.ReadToEnd();
            return string.IsNullOrWhiteSpace(body) ? null : Json.Deserialize<T>(body);
        }
    }

    private static void AddCorsHeaders(HttpListenerResponse response)
    {
        response.Headers["Access-Control-Allow-Origin"] = "*";
        response.Headers["Access-Control-Allow-Methods"] = "GET, POST, OPTIONS";
        response.Headers["Access-Control-Allow-Headers"] = "Authorization, Content-Type";
    }

    private static void WriteJson(HttpListenerResponse response, object payload)
    {
        byte[] bytes = Encoding.UTF8.GetBytes(Json.Serialize(payload));
        response.ContentType = "application/json; charset=utf-8";
        response.ContentLength64 = bytes.Length;
        response.OutputStream.Write(bytes, 0, bytes.Length);
        response.OutputStream.Close();
    }
}
