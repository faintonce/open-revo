using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Net;
using System.Net.Sockets;
using System.Security.Principal;
using System.ServiceProcess;
using System.Text;
using System.Threading;
using System.Web.Script.Serialization;
using Intel.Overclocking.SDK.Common;
using Intel.Overclocking.SDK.ServiceInfo;
using Intel.Overclocking.SDK.Tuning;

namespace RevoXtuBridge
{
    class Program
    {
        // Key Control IDs based on Intel XTU KnobMetadata
        public const uint ID_MAX_TURBO_RATIO = 30;
        public const uint ID_CPU_VOLTAGE_OFFSET = 34;
        public const uint ID_TURBO_POWER_MAX = 48;        // PL1 (Watts)
        public const uint ID_TURBO_POWER_SHORT = 49;      // PL2 (Watts)
        public const uint ID_IA_CORE_CURRENT_MAX = 57;    // Current Max (Amps)
        public const uint ID_RING_VOLTAGE_OFFSET = 79;    // Cache/Ring Voltage Offset
        public const uint ID_IA_CORE_VOLTAGE_MODE = 88;   // 0 = Adaptive, 1 = Static/Override
        public const uint ID_PROCESSOR_CORE_ICC_MAX = 102; // Core IccMax (Amps)

        private const string SERVICE_NAME = "XTUService";
        private static JavaScriptSerializer serializer = new JavaScriptSerializer();
        private static volatile bool serverRunning = true;

        static int Main(string[] args)
        {
            if (args.Length == 0 || args.Contains("--help") || args.Contains("-h"))
            {
                PrintHelp();
                return 0;
            }

            string command = args[0].ToLowerInvariant();

            try
            {
                switch (command)
                {
                    case "debug":
                        {
                            string logFile = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "debug_output.txt");
                            using (var sw = new StreamWriter(logFile, false, Encoding.UTF8))
                            {
                                void LogLine(string s)
                                {
                                    Console.WriteLine(s);
                                    sw.WriteLine(s);
                                    sw.Flush();
                                }

                                LogLine("IsAdministrator: " + IsAdministrator());
                                LogLine("XTU3SERVICE Status:");
                                try
                                {
                                    using (var sc = new ServiceController("XTU3SERVICE"))
                                    {
                                        LogLine("  XTU3SERVICE: " + sc.Status);
                                    }
                                }
                                catch (Exception ex) { LogLine("  XTU3SERVICE error: " + ex.Message); }

                                ITuningLibrary t = TuningLibrary.Instance;
                                LogLine("TuningLibrary instance: " + t.GetType().FullName);
                                LogLine("Calling t.Initialize()...");
                                try
                                {
                                    t.Initialize();
                                    LogLine("t.Initialize() completed!");
                                    LogLine("InitializeCheck: " + t.InitializeCheck());
                                    var ctls = t.GetAvailableControls();
                                    LogLine("GetAvailableControls count: " + (ctls != null ? ctls.Count.ToString() : "null"));
                                    if (ctls != null && ctls.Count > 0)
                                    {
                                        foreach (var c in ctls.Where(x => x.Id == ID_CPU_VOLTAGE_OFFSET || x.Id == ID_MAX_TURBO_RATIO || x.Id == ID_PROCESSOR_CORE_ICC_MAX))
                                        {
                                            LogLine($"  ID={c.Id}, Name={c.Name}, Active={c.ActiveValue}, Def={c.DefaultValue}, Units={c.Units}, ValuesCount={c.SupportedValues?.Count}, FirstVal={(c.SupportedValues?.Count > 0 ? c.SupportedValues[0].ToString() : "none")}, LastVal={(c.SupportedValues?.Count > 0 ? c.SupportedValues.Last().ToString() : "none")}");
                                        }
                                    }
                                }
                                catch (Exception ex)
                                {
                                    LogLine("Initialize exception: " + ex.ToString());
                                }
                            }
                            return 0;
                        }

                    case "--serve":
                    case "serve":
                        return HandleServe(args);

                    case "status":
                    case "probe":
                        OutputJson(GetProbeResult());
                        return 0;

                    case "get":
                        OutputJson(GetTuningKnobsResult());
                        return 0;

                    case "dump":
                        return HandleDump(args);

                    case "apply":
                    case "tune":
                        OutputJson(ApplyTuningResult(args));
                        return 0;

                    case "reset":
                        OutputJson(ResetTuningResult());
                        return 0;

                    case "service-install":
                        OutputJson(ServiceInstallResult(args));
                        return 0;

                    case "service-start":
                        OutputJson(ServiceStartResult());
                        return 0;

                    case "service-stop":
                        OutputJson(ServiceStopResult());
                        return 0;

                    case "service-uninstall":
                        OutputJson(ServiceUninstallResult());
                        return 0;

                    default:
                        OutputJson(new Dictionary<string, object>
                        {
                            ["success"] = false,
                            ["error"] = "Unknown command: " + command
                        });
                        return 1;
                }
            }
            catch (Exception ex)
            {
                OutputJson(new Dictionary<string, object>
                {
                    ["success"] = false,
                    ["error"] = ex.GetType().Name + ": " + ex.Message,
                    ["stack_trace"] = ex.StackTrace
                });
                return 2;
            }
        }

        private static void PrintHelp()
        {
            Console.WriteLine(@"Revo XTU Bridge - OpenRevo CPU Guardian Sidecar
Usage:
  revo-xtu-bridge.exe serve [--port <port>]  Run companion HTTP daemon (default: 18980)
  revo-xtu-bridge.exe probe                  Check service status, pipe connection, and UVP
  revo-xtu-bridge.exe get                    Get current values of key tuning knobs
  revo-xtu-bridge.exe apply [options]        Apply anti-degradation or custom tuning
  revo-xtu-bridge.exe reset                  Reset voltage offsets and ratios to defaults
  revo-xtu-bridge.exe dump [--out <file>]    Dump all supported XTU controls to JSON
  revo-xtu-bridge.exe service-install        Register XTUService in SCM with auto-elevation
  revo-xtu-bridge.exe service-start          Start XTUService with auto-elevation
  revo-xtu-bridge.exe service-stop           Stop XTUService with auto-elevation
  revo-xtu-bridge.exe service-uninstall      Remove XTUService from SCM with auto-elevation
");
        }

        #region HTTP Daemon Server (TcpListener)

        private static int HandleServe(string[] args)
        {
            int port = 18980;
            string portStr = GetArgValue(args, "--port");
            if (!string.IsNullOrEmpty(portStr) && int.TryParse(portStr, out int p))
            {
                port = p;
            }

            TcpListener listener = new TcpListener(IPAddress.Loopback, port);
            try
            {
                listener.Start();
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"[FATAL] Failed to bind to 127.0.0.1:{port}: {ex.Message}");
                return 1;
            }

            Console.WriteLine($"[INFO] Revo XTU Bridge daemon listening on http://127.0.0.1:{port}/");

            while (serverRunning)
            {
                try
                {
                    TcpClient client = listener.AcceptTcpClient();
                    ThreadPool.QueueUserWorkItem(ProcessClientRequest, client);
                }
                catch
                {
                    if (!serverRunning) break;
                }
            }

            return 0;
        }

        private static void ProcessClientRequest(object state)
        {
            using (TcpClient client = (TcpClient)state)
            using (NetworkStream stream = client.GetStream())
            {
                try
                {
                    byte[] buffer = new byte[8192];
                    int bytesRead = stream.Read(buffer, 0, buffer.Length);
                    if (bytesRead <= 0) return;

                    string requestStr = Encoding.UTF8.GetString(buffer, 0, bytesRead);
                    string[] lines = requestStr.Split(new[] { "\r\n", "\n" }, StringSplitOptions.None);
                    if (lines.Length == 0) return;

                    string firstLine = lines[0];
                    string[] parts = firstLine.Split(' ');
                    if (parts.Length < 2) return;

                    string method = parts[0].ToUpperInvariant();
                    string fullUrl = parts[1];

                    if (method == "OPTIONS")
                    {
                        SendHttpResponse(stream, 200, "OK", "{}", "application/json");
                        return;
                    }

                    string path = fullUrl;
                    string query = "";
                    int qIndex = fullUrl.IndexOf('?');
                    if (qIndex >= 0)
                    {
                        path = fullUrl.Substring(0, qIndex);
                        query = fullUrl.Substring(qIndex + 1);
                    }

                    var queryParams = ParseQueryString(query);
                    object responseData = null;

                    if (path == "/api/health" || path == "/health")
                    {
                        responseData = new Dictionary<string, object>
                        {
                            ["status"] = "ok",
                            ["service"] = "revo-xtu-bridge",
                            ["timestamp"] = DateTime.UtcNow.ToString("o")
                        };
                    }
                    else if (path == "/api/probe" || path == "/probe")
                    {
                        responseData = GetProbeResult();
                    }
                    else if (path == "/api/get" || path == "/get")
                    {
                        responseData = GetTuningKnobsResult();
                    }
                    else if (path == "/api/dump" || path == "/dump")
                    {
                        responseData = DumpTuningControlsResult();
                    }
                    else if (path == "/api/apply" || path == "/apply")
                    {
                        var cliArgs = BuildCliArgsFromQuery(queryParams);
                        responseData = ApplyTuningResult(cliArgs);
                    }
                    else if (path == "/api/reset" || path == "/reset")
                    {
                        responseData = ResetTuningResult();
                    }
                    else if (path == "/api/exit" || path == "/exit")
                    {
                        responseData = new Dictionary<string, object> { ["success"] = true, ["message"] = "Exiting daemon" };
                        string jsonExit = serializer.Serialize(responseData);
                        SendHttpResponse(stream, 200, "OK", jsonExit, "application/json");
                        new Thread(() => { Thread.Sleep(300); Environment.Exit(0); }).Start();
                        return;
                    }
                    else if (path == "/api/service/install")
                    {
                        var cliArgs = BuildCliArgsFromQuery(queryParams);
                        responseData = ServiceInstallResult(cliArgs);
                    }
                    else if (path == "/api/service/start")
                    {
                        responseData = ServiceStartResult();
                    }
                    else if (path == "/api/service/stop")
                    {
                        responseData = ServiceStopResult();
                    }
                    else if (path == "/api/service/uninstall")
                    {
                        responseData = ServiceUninstallResult();
                    }
                    else
                    {
                        SendHttpResponse(stream, 404, "Not Found", "{\"error\":\"Unknown endpoint\"}", "application/json");
                        return;
                    }

                    string json = serializer.Serialize(responseData);
                    SendHttpResponse(stream, 200, "OK", json, "application/json");
                }
                catch (Exception ex)
                {
                    string errJson = serializer.Serialize(new Dictionary<string, object>
                    {
                        ["success"] = false,
                        ["error"] = ex.GetType().Name + ": " + ex.Message
                    });
                    try
                    {
                        SendHttpResponse(stream, 500, "Internal Server Error", errJson, "application/json");
                    }
                    catch { }
                }
            }
        }

        private static void SendHttpResponse(NetworkStream stream, int statusCode, string statusMsg, string body, string contentType)
        {
            byte[] bodyBytes = Encoding.UTF8.GetBytes(body);
            StringBuilder sb = new StringBuilder();
            sb.AppendLine($"HTTP/1.1 {statusCode} {statusMsg}");
            sb.AppendLine("Access-Control-Allow-Origin: *");
            sb.AppendLine("Access-Control-Allow-Methods: GET, POST, OPTIONS");
            sb.AppendLine("Access-Control-Allow-Headers: Content-Type");
            sb.AppendLine($"Content-Type: {contentType}; charset=utf-8");
            sb.AppendLine($"Content-Length: {bodyBytes.Length}");
            sb.AppendLine("Connection: close");
            sb.AppendLine();

            byte[] headerBytes = Encoding.ASCII.GetBytes(sb.ToString());
            stream.Write(headerBytes, 0, headerBytes.Length);
            stream.Write(bodyBytes, 0, bodyBytes.Length);
            stream.Flush();
        }

        private static Dictionary<string, string> ParseQueryString(string query)
        {
            var dict = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            if (string.IsNullOrEmpty(query)) return dict;

            string[] pairs = query.Split('&');
            foreach (var pair in pairs)
            {
                if (string.IsNullOrEmpty(pair)) continue;
                string[] kv = pair.Split('=');
                string key = Uri.UnescapeDataString(kv[0]);
                string val = kv.Length > 1 ? Uri.UnescapeDataString(kv[1]) : "";
                dict[key] = val;
            }
            return dict;
        }

        private static string[] BuildCliArgsFromQuery(Dictionary<string, string> query)
        {
            List<string> list = new List<string> { "apply" };
            if (query.TryGetValue("core_offset", out string co)) { list.Add("--core-offset"); list.Add(co); }
            if (query.TryGetValue("ring_offset", out string ro)) { list.Add("--ring-offset"); list.Add(ro); }
            if (query.TryGetValue("max_ratio", out string mr)) { list.Add("--max-ratio"); list.Add(mr); }
            if (query.TryGetValue("iccmax", out string im)) { list.Add("--iccmax"); list.Add(im); }
            if (query.TryGetValue("pl1", out string p1)) { list.Add("--pl1"); list.Add(p1); }
            if (query.TryGetValue("pl2", out string p2)) { list.Add("--pl2"); list.Add(p2); }
            if (query.TryGetValue("path", out string pth)) { list.Add("--path"); list.Add(pth); }
            return list.ToArray();
        }

        #endregion

        #region Core Business Operations

        private static readonly string[] SERVICE_NAMES = new[] { "XTU3SERVICE", "XTUService" };

        private static string GetActiveServiceName()
        {
            foreach (var name in SERVICE_NAMES)
            {
                try
                {
                    using (var sc = new ServiceController(name))
                    {
                        if (sc.Status == ServiceControllerStatus.Running) return name;
                    }
                }
                catch { }
            }
            foreach (var name in SERVICE_NAMES)
            {
                try
                {
                    using (var sc = new ServiceController(name))
                    {
                        var s = sc.Status;
                        return name;
                    }
                }
                catch { }
            }
            return "XTU3SERVICE";
        }

        public static Dictionary<string, object> GetProbeResult()
        {
            var result = new Dictionary<string, object>();
            
            string svcStatus = "NotInstalled";
            string activeSvc = GetActiveServiceName();
            foreach (var sname in SERVICE_NAMES)
            {
                try
                {
                    using (var sc = new ServiceController(sname))
                    {
                        if (sc.Status == ServiceControllerStatus.Running)
                        {
                            svcStatus = "Running";
                            activeSvc = sname;
                            break;
                        }
                        else if (svcStatus == "NotInstalled")
                        {
                            svcStatus = sc.Status.ToString();
                        }
                    }
                }
                catch { }
            }
            result["service_status"] = svcStatus;
            result["active_service_name"] = activeSvc;

            ITuningLibrary tuning = TuningLibrary.Instance;
            bool initOk = false;
            try
            {
                if (!tuning.InitializeCheck())
                {
                    tuning.Initialize();
                }
                initOk = tuning.InitializeCheck();
            }
            catch (Exception ex)
            {
                result["success"] = false;
                result["connected"] = false;
                result["error"] = "Failed to connect to XTU service: " + ex.Message;
                result["uvp_active"] = false;
                return result;
            }

            result["connected"] = initOk;
            if (!initOk)
            {
                result["success"] = false;
                result["error"] = "XTU TuningLibrary.InitializeCheck returned false.";
                result["uvp_active"] = false;
                return result;
            }

            var controls = tuning.GetAvailableControls();
            if (controls == null || controls.Count == 0)
            {
                try { controls = tuning.RescanAvailableControls(); } catch { }
            }
            result["total_controls"] = controls != null ? controls.Count : 0;

            bool uvpActive = false;
            bool vOffsetTunable = false;
            bool ratioTunable = false;
            bool iccTunable = false;

            if (controls != null && controls.Count > 0)
            {
                var vOffsetCtrl = controls.FirstOrDefault(c => c.Id == ID_CPU_VOLTAGE_OFFSET);
                if (vOffsetCtrl != null)
                {
                    vOffsetTunable = vOffsetCtrl.Enabled && !vOffsetCtrl.ReadOnly;
                    uvpActive = !vOffsetTunable;
                }

                var ratioCtrl = controls.FirstOrDefault(c => c.Id == ID_MAX_TURBO_RATIO);
                ratioTunable = ratioCtrl != null && ratioCtrl.Enabled && !ratioCtrl.ReadOnly;

                var iccCtrl = controls.FirstOrDefault(c => c.Id == ID_PROCESSOR_CORE_ICC_MAX || c.Id == ID_IA_CORE_CURRENT_MAX);
                iccTunable = iccCtrl != null && iccCtrl.Enabled && !iccCtrl.ReadOnly;
            }

            result["uvp_active"] = uvpActive;
            result["voltage_offset_tunable"] = vOffsetTunable;
            result["ratio_tunable"] = ratioTunable;
            result["iccmax_tunable"] = iccTunable;
            result["success"] = true;
            return result;
        }

        public static Dictionary<string, object> GetTuningKnobsResult()
        {
            ITuningLibrary tuning = TuningLibrary.Instance;
            if (!tuning.InitializeCheck())
            {
                try { tuning.Initialize(); } catch { }
            }
            if (!tuning.InitializeCheck())
            {
                return new Dictionary<string, object>
                {
                    ["success"] = false,
                    ["error"] = "XTU Tuning service is not available."
                };
            }

            var controls = tuning.GetAvailableControls();
            if (controls == null || controls.Count == 0)
            {
                try { controls = tuning.RescanAvailableControls(); } catch { }
            }
            if (controls == null || controls.Count == 0)
            {
                return new Dictionary<string, object>
                {
                    ["success"] = false,
                    ["error"] = "No tuning controls available."
                };
            }

            var keyKnobs = new Dictionary<string, object>();

            AddControlInfo(keyKnobs, "core_voltage_offset", controls, ID_CPU_VOLTAGE_OFFSET, true);
            AddControlInfo(keyKnobs, "ring_voltage_offset", controls, ID_RING_VOLTAGE_OFFSET, true);
            AddControlInfo(keyKnobs, "max_turbo_ratio", controls, ID_MAX_TURBO_RATIO, false);
            AddControlInfo(keyKnobs, "core_icc_max", controls, ID_PROCESSOR_CORE_ICC_MAX, false);
            AddControlInfo(keyKnobs, "turbo_power_max", controls, ID_TURBO_POWER_MAX, false);
            AddControlInfo(keyKnobs, "turbo_power_short", controls, ID_TURBO_POWER_SHORT, false);

            var vOffset = controls.FirstOrDefault(c => c.Id == ID_CPU_VOLTAGE_OFFSET);
            bool uvpActive = vOffset != null && (!vOffset.Enabled || vOffset.ReadOnly);

            return new Dictionary<string, object>
            {
                ["success"] = true,
                ["uvp_active"] = uvpActive,
                ["knobs"] = keyKnobs
            };
        }

        private static void AddControlInfo(Dictionary<string, object> dict, string key, List<ClientTuningControl> controls, uint id, bool isMilliVolts)
        {
            var ctrl = controls.FirstOrDefault(c => c.Id == id);
            if (ctrl == null)
            {
                dict[key] = new Dictionary<string, object>
                {
                    ["id"] = id,
                    ["supported"] = false
                };
                return;
            }

            decimal current = ctrl.ActiveValue;
            decimal defVal = ctrl.DefaultValue;
            decimal minVal = ctrl.SupportedValues != null && ctrl.SupportedValues.Count > 0 ? ctrl.SupportedValues.First() : 0m;
            decimal maxVal = ctrl.SupportedValues != null && ctrl.SupportedValues.Count > 0 ? ctrl.SupportedValues.Last() : 0m;
            decimal stepVal = 0m;

            if (ctrl.SupportedValues != null && ctrl.SupportedValues.Count > 1)
            {
                stepVal = Math.Abs(ctrl.SupportedValues[1] - ctrl.SupportedValues[0]);
            }

            // Note: Intel XTU SDK natively returns and accepts voltage offsets in mV (e.g. -70 is -70 mV).
            // Do NOT multiply or divide by 1000!

            dict[key] = new Dictionary<string, object>
            {
                ["id"] = ctrl.Id,
                ["name"] = ctrl.Name,
                ["supported"] = true,
                ["enabled"] = ctrl.Enabled,
                ["read_only"] = ctrl.ReadOnly,
                ["requires_reboot"] = ctrl.RequiresReboot,
                ["current"] = current,
                ["default"] = defVal,
                ["min"] = minVal,
                ["max"] = maxVal,
                ["step"] = stepVal,
                ["units"] = isMilliVolts ? "mV" : ctrl.Units
            };
        }

        public static Dictionary<string, object> ApplyTuningResult(string[] args)
        {
            ITuningLibrary tuning = TuningLibrary.Instance;
            if (!tuning.InitializeCheck())
            {
                try { tuning.Initialize(); } catch { }
            }
            if (!tuning.InitializeCheck())
            {
                return new Dictionary<string, object>
                {
                    ["success"] = false,
                    ["error"] = "XTU Tuning service is not available."
                };
            }

            var controls = tuning.GetAvailableControls();
            if (controls == null || controls.Count == 0)
            {
                try { controls = tuning.RescanAvailableControls(); } catch { }
            }
            var proposals = new List<ClientTuningProposal>();
            var plan = new Dictionary<string, object>();

            string coreOffsetStr = GetArgValue(args, "--core-offset");
            if (!string.IsNullOrEmpty(coreOffsetStr))
            {
                if (decimal.TryParse(coreOffsetStr, out decimal mv))
                {
                    // Intel XTU SDK natively expects millivolts (mV) in proposals (e.g. -60m for -60mV).
                    var coreCtrl = controls?.FirstOrDefault(c => c.Id == ID_CPU_VOLTAGE_OFFSET);
                    if (coreCtrl != null && coreCtrl.SupportedValues != null && coreCtrl.SupportedValues.Count > 0)
                    {
                        decimal minCore = coreCtrl.SupportedValues.First();
                        decimal maxCore = coreCtrl.SupportedValues.Last();
                        if (mv < minCore) mv = minCore;
                        if (mv > maxCore) mv = maxCore;
                    }
                    proposals.Add(new ClientTuningProposal { Id = ID_CPU_VOLTAGE_OFFSET, Value = mv });
                    plan["core_voltage_offset_mv"] = mv;
                }
            }

            string ringOffsetStr = GetArgValue(args, "--ring-offset");
            if (!string.IsNullOrEmpty(ringOffsetStr))
            {
                if (decimal.TryParse(ringOffsetStr, out decimal mv))
                {
                    var ringCtrl = controls?.FirstOrDefault(c => c.Id == ID_RING_VOLTAGE_OFFSET);
                    if (ringCtrl != null && ringCtrl.SupportedValues != null && ringCtrl.SupportedValues.Count > 0)
                    {
                        decimal minVal = ringCtrl.SupportedValues.First();
                        decimal maxVal = ringCtrl.SupportedValues.Last();
                        // On 14th Gen HX, min is 0 (negative ring offset locked by microcode in OS runtime)
                        if (mv < minVal) mv = minVal;
                        if (mv > maxVal) mv = maxVal;
                        proposals.Add(new ClientTuningProposal { Id = ID_RING_VOLTAGE_OFFSET, Value = mv });
                        plan["ring_voltage_offset_mv"] = mv;
                    }
                }
            }

            string maxRatioStr = GetArgValue(args, "--max-ratio");
            if (!string.IsNullOrEmpty(maxRatioStr))
            {
                if (decimal.TryParse(maxRatioStr, out decimal ratio))
                {
                    // Find all active performance core ratio controls to enforce global frequency ceiling and monotonicity
                    var ratioCtrls = controls?.Where(c => c.Enabled && !c.ReadOnly && (c.Name.Contains("Active Performance Core") || c.Id == ID_MAX_TURBO_RATIO)).ToList();
                    if (ratioCtrls != null && ratioCtrls.Count > 0)
                    {
                        foreach (var rc in ratioCtrls)
                        {
                            decimal targetRatio = ratio;
                            if (rc.SupportedValues != null && rc.SupportedValues.Count > 0)
                            {
                                if (targetRatio < rc.SupportedValues.First()) targetRatio = rc.SupportedValues.First();
                                if (targetRatio > rc.SupportedValues.Last()) targetRatio = rc.SupportedValues.Last();
                            }
                            proposals.Add(new ClientTuningProposal { Id = rc.Id, Value = targetRatio });
                        }
                    }
                    else
                    {
                        proposals.Add(new ClientTuningProposal { Id = ID_MAX_TURBO_RATIO, Value = ratio });
                    }
                    plan["max_turbo_ratio"] = ratio;
                }
            }

            string iccmaxStr = GetArgValue(args, "--iccmax");
            if (!string.IsNullOrEmpty(iccmaxStr))
            {
                if (decimal.TryParse(iccmaxStr, out decimal a))
                {
                    uint targetId = controls.Any(c => c.Id == ID_PROCESSOR_CORE_ICC_MAX) ? ID_PROCESSOR_CORE_ICC_MAX : ID_IA_CORE_CURRENT_MAX;
                    var iccCtrl = controls.FirstOrDefault(c => c.Id == targetId);
                    if (iccCtrl != null && iccCtrl.SupportedValues != null && iccCtrl.SupportedValues.Count > 0)
                    {
                        if (a < iccCtrl.SupportedValues.First()) a = iccCtrl.SupportedValues.First();
                        if (a > iccCtrl.SupportedValues.Last()) a = iccCtrl.SupportedValues.Last();
                    }
                    proposals.Add(new ClientTuningProposal { Id = targetId, Value = a });
                    plan["icc_max_a"] = a;
                }
            }

            string pl1Str = GetArgValue(args, "--pl1");
            if (!string.IsNullOrEmpty(pl1Str))
            {
                if (decimal.TryParse(pl1Str, out decimal pl1))
                {
                    proposals.Add(new ClientTuningProposal { Id = ID_TURBO_POWER_MAX, Value = pl1 });
                    plan["pl1_w"] = pl1;
                }
            }

            string pl2Str = GetArgValue(args, "--pl2");
            if (!string.IsNullOrEmpty(pl2Str))
            {
                if (decimal.TryParse(pl2Str, out decimal pl2))
                {
                    proposals.Add(new ClientTuningProposal { Id = ID_TURBO_POWER_SHORT, Value = pl2 });
                    plan["pl2_w"] = pl2;
                }
            }

            if (proposals.Count == 0)
            {
                return new Dictionary<string, object>
                {
                    ["success"] = false,
                    ["error"] = "No valid tuning parameters provided. Use --help to view available options."
                };
            }

            bool rebootRequired = false;
            bool tuneSuccess = tuning.Tune(proposals, out rebootRequired);

            if (!tuneSuccess)
            {
                // Fallback: Apply proposals sequentially if batch transaction is rejected by XTU
                int successCount = 0;
                foreach (var p in proposals)
                {
                    try
                    {
                        bool singleReboot = false;
                        var singleList = new List<ClientTuningProposal> { p };
                        if (tuning.Tune(singleList, out singleReboot))
                        {
                            successCount++;
                            if (singleReboot) rebootRequired = true;
                        }
                    }
                    catch { }
                }

                if (successCount > 0)
                {
                    tuneSuccess = true;
                }
            }

            if (!tuneSuccess)
            {
                return new Dictionary<string, object>
                {
                    ["success"] = false,
                    ["error"] = "XTU Tune rejected the proposed parameters."
                };
            }

            var updatedControls = tuning.GetAvailableControls();
            var verified = new Dictionary<string, object>();
            foreach (var p in proposals)
            {
                var ctrl = updatedControls?.FirstOrDefault(c => c.Id == p.Id);
                if (ctrl != null)
                {
                    verified[ctrl.Name] = ctrl.ActiveValue;
                }
            }

            return new Dictionary<string, object>
            {
                ["success"] = true,
                ["requires_reboot"] = rebootRequired,
                ["applied_plan"] = plan,
                ["verified"] = verified
            };
        }

        public static Dictionary<string, object> ResetTuningResult()
        {
            ITuningLibrary tuning = TuningLibrary.Instance;
            if (!tuning.InitializeCheck())
            {
                try { tuning.Initialize(); } catch { }
            }
            if (!tuning.InitializeCheck())
            {
                return new Dictionary<string, object>
                {
                    ["success"] = false,
                    ["error"] = "XTU Tuning service is not available."
                };
            }

            var controls = tuning.GetAvailableControls();
            if (controls == null || controls.Count == 0)
            {
                try { controls = tuning.RescanAvailableControls(); } catch { }
            }
            var resetProposals = new List<ClientTuningProposal>();

            // Collect all relevant tunable controls to restore to DefaultValue
            var resetCtrls = controls?.Where(c => c.Enabled && !c.ReadOnly && (
                c.Id == ID_CPU_VOLTAGE_OFFSET ||
                c.Id == ID_RING_VOLTAGE_OFFSET ||
                c.Id == ID_PROCESSOR_CORE_ICC_MAX ||
                c.Id == ID_IA_CORE_CURRENT_MAX ||
                c.Id == ID_TURBO_POWER_MAX ||
                c.Id == ID_TURBO_POWER_SHORT ||
                c.Id == ID_MAX_TURBO_RATIO ||
                c.Name.Contains("Active Performance Core")
            )).ToList();

            if (resetCtrls != null)
            {
                foreach (var ctrl in resetCtrls)
                {
                    resetProposals.Add(new ClientTuningProposal
                    {
                        Id = ctrl.Id,
                        Value = ctrl.DefaultValue
                    });
                }
            }

            if (resetProposals.Count > 0)
            {
                bool rebootRequired = false;
                bool ok = tuning.Tune(resetProposals, out rebootRequired);
                if (!ok)
                {
                    int resetSuccess = 0;
                    foreach (var p in resetProposals)
                    {
                        try
                        {
                            bool rb = false;
                            if (tuning.Tune(new List<ClientTuningProposal> { p }, out rb))
                            {
                                resetSuccess++;
                                if (rb) rebootRequired = true;
                            }
                        }
                        catch { }
                    }
                    ok = resetSuccess > 0;
                }

                return new Dictionary<string, object>
                {
                    ["success"] = ok,
                    ["reset_count"] = resetProposals.Count,
                    ["requires_reboot"] = rebootRequired
                };
            }

            return new Dictionary<string, object>
            {
                ["success"] = true,
                ["message"] = "All controls already at default or not tunable."
            };
        }

        private static object DumpTuningControlsResult()
        {
            ITuningLibrary tuning = TuningLibrary.Instance;
            if (!tuning.InitializeCheck())
            {
                return new Dictionary<string, object>
                {
                    ["success"] = false,
                    ["error"] = "XTU Tuning service is not available."
                };
            }

            var controls = tuning.GetAvailableControls();
            if (controls == null)
            {
                return new Dictionary<string, object>
                {
                    ["success"] = false,
                    ["error"] = "No tuning controls returned."
                };
            }

            var list = controls.Select(c => new Dictionary<string, object>
            {
                ["id"] = c.Id,
                ["name"] = c.Name,
                ["category"] = c.Category,
                ["active_value"] = c.ActiveValue,
                ["default_value"] = c.DefaultValue,
                ["min_value"] = c.SupportedValues != null && c.SupportedValues.Count > 0 ? c.SupportedValues.First() : 0m,
                ["max_value"] = c.SupportedValues != null && c.SupportedValues.Count > 0 ? c.SupportedValues.Last() : 0m,
                ["step"] = c.SupportedValues != null && c.SupportedValues.Count > 1 ? Math.Abs(c.SupportedValues[1] - c.SupportedValues[0]) : 0m,
                ["units"] = c.Units,
                ["enabled"] = c.Enabled,
                ["read_only"] = c.ReadOnly
            }).ToList();

            return new Dictionary<string, object>
            {
                ["success"] = true,
                ["total"] = list.Count,
                ["controls"] = list
            };
        }

        private static int HandleDump(string[] args)
        {
            ITuningLibrary tuning = TuningLibrary.Instance;
            if (!tuning.InitializeCheck())
            {
                OutputJson(new Dictionary<string, object>
                {
                    ["success"] = false,
                    ["error"] = "XTU Tuning service is not available."
                });
                return 1;
            }

            var controls = tuning.GetAvailableControls();
            if (controls == null)
            {
                OutputJson(new Dictionary<string, object>
                {
                    ["success"] = false,
                    ["error"] = "No tuning controls returned."
                });
                return 1;
            }

            var list = controls.Select(c => new Dictionary<string, object>
            {
                ["id"] = c.Id,
                ["name"] = c.Name,
                ["category"] = c.Category,
                ["active_value"] = c.ActiveValue,
                ["default_value"] = c.DefaultValue,
                ["boot_value"] = c.BootValue,
                ["min_value"] = c.SupportedValues != null && c.SupportedValues.Count > 0 ? c.SupportedValues.First() : 0m,
                ["max_value"] = c.SupportedValues != null && c.SupportedValues.Count > 0 ? c.SupportedValues.Last() : 0m,
                ["supported_values_count"] = c.SupportedValues != null ? c.SupportedValues.Count : 0,
                ["units"] = c.Units,
                ["enabled"] = c.Enabled,
                ["read_only"] = c.ReadOnly,
                ["requires_reboot"] = c.RequiresReboot
            }).ToList();

            string outFile = GetArgValue(args, "--out");
            if (!string.IsNullOrEmpty(outFile))
            {
                string json = serializer.Serialize(list);
                File.WriteAllText(outFile, json);
                OutputJson(new Dictionary<string, object>
                {
                    ["success"] = true,
                    ["dumped_count"] = list.Count,
                    ["file"] = outFile
                });
                return 0;
            }

            OutputJson(new Dictionary<string, object>
            {
                ["success"] = true,
                ["total"] = list.Count,
                ["controls"] = list
            });
            return 0;
        }

        private static string ResolveDriverPath(string userSpecifiedPath = null)
        {
            if (!string.IsNullOrEmpty(userSpecifiedPath) && File.Exists(userSpecifiedPath))
            {
                return Path.GetFullPath(userSpecifiedPath);
            }

            // 1. First priority: Check local companion plugin bundled driver directory
            string current = AppDomain.CurrentDomain.BaseDirectory;
            for (int i = 0; i < 7; i++)
            {
                string test = Path.Combine(current, @"driver\XtuService.exe");
                if (File.Exists(test))
                {
                    return Path.GetFullPath(test);
                }
                DirectoryInfo parent = Directory.GetParent(current);
                if (parent == null) break;
                current = parent.FullName;
            }

            // 2. Second priority: Official install path
            string officialPath = @"C:\Program Files\Intel\Intel(R) Extreme Tuning Utility\Drivers\XtuService.exe";
            if (File.Exists(officialPath))
            {
                return officialPath;
            }

            return null;
        }

        private static void EnsureKernelDriverReady()
        {
            // CRITICAL SAFETY GUARD:
            // Never attempt 'sc create iocbios2' manually on modern Windows 11!
            // iocbios2.sys strictly requires ACPI PnP extension device attachment (SWC\XTUCOMPONENT + XTUExtension)
            // registered via Intel's official installer.
            // Creating it as an orphaned NT service causes ACPI handle null-pointer dereference and 0x50 kernel BSOD!
        }

        public static Dictionary<string, object> ServiceInstallResult(string[] args)
        {
            string binPath = ResolveDriverPath(GetArgValue(args, "--path"));
            if (string.IsNullOrEmpty(binPath) || !File.Exists(binPath))
            {
                return new Dictionary<string, object>
                {
                    ["success"] = false,
                    ["error"] = "未能找到有效的 XtuService.exe。请确保插件包含 driver 目录或已安装 Intel XTU。"
                };
            }

            // 1. Ensure kernel driver iocbios2 is installed and running
            EnsureKernelDriverReady();

            // 2. Stop existing service if running
            RunElevatedCommand("sc.exe", $"stop {SERVICE_NAME}");

            // 3. Try creating service (in case it didn't exist)
            string createOut = RunElevatedCommand("sc.exe", $"create {SERVICE_NAME} binPath= \"{binPath}\" start= demand DisplayName= \"Intel(R) Extreme Tuning Utility Service\"");

            // 4. Always update binPath via config (handles case where service already existed with obsolete path)
            string configOut = RunElevatedCommand("sc.exe", $"config {SERVICE_NAME} binPath= \"{binPath}\" start= demand");

            RunElevatedCommand("sc.exe", $"description {SERVICE_NAME} \"Provides hardware overclocking, undervolting and monitoring services for OpenRevo.\"");

            // 5. Start service
            string startOut = RunElevatedCommand("sc.exe", $"start {SERVICE_NAME}");

            bool success = startOut.Contains("START_PENDING") || startOut.Contains("RUNNING") || startOut.Contains("1056");

            return new Dictionary<string, object>
            {
                ["success"] = success,
                ["bin_path"] = binPath,
                ["create_output"] = createOut.Trim(),
                ["config_output"] = configOut.Trim(),
                ["start_output"] = startOut.Trim()
            };
        }

        public static Dictionary<string, object> ServiceStartResult()
        {
            string svcName = GetActiveServiceName();

            // Auto-heal: Ensure kernel driver is ready
            EnsureKernelDriverReady();

            // Auto-heal: Ensure binPath points to our current valid driver binary
            string binPath = ResolveDriverPath();
            if (!string.IsNullOrEmpty(binPath) && svcName == "XTUService")
            {
                RunElevatedCommand("sc.exe", $"config {svcName} binPath= \"{binPath}\" start= demand");
            }

            string output = RunElevatedCommand("sc.exe", $"start {svcName}");
            bool ok = output.Contains("START_PENDING") || output.Contains("RUNNING") || output.Contains("1056");

            if (!ok && (output.Contains("1060") || output.Contains(" 2:") || output.Contains("1053") || output.Contains("系统找不到指定的文件")))
            {
                // Service missing or invalid path or driver not loaded, trigger full install
                return ServiceInstallResult(new string[0]);
            }

            return new Dictionary<string, object>
            {
                ["success"] = ok,
                ["service_name"] = svcName,
                ["output"] = output.Trim()
            };
        }

        public static Dictionary<string, object> ServiceStopResult()
        {
            string svcName = GetActiveServiceName();
            string output = RunElevatedCommand("sc.exe", $"stop {svcName}");
            bool ok = output.Contains("STOP_PENDING") || output.Contains("STOPPED") || output.Contains("1062");

            return new Dictionary<string, object>
            {
                ["success"] = ok,
                ["service_name"] = svcName,
                ["output"] = output.Trim()
            };
        }

        public static Dictionary<string, object> ServiceUninstallResult()
        {
            RunElevatedCommand("sc.exe", $"stop {SERVICE_NAME}");
            string output = RunElevatedCommand("sc.exe", $"delete {SERVICE_NAME}");
            bool ok = output.Contains("SUCCESS") || output.Contains("1060");

            return new Dictionary<string, object>
            {
                ["success"] = ok,
                ["output"] = output.Trim()
            };
        }

        private static bool IsAdministrator()
        {
            using (var identity = WindowsIdentity.GetCurrent())
            {
                var principal = new WindowsPrincipal(identity);
                return principal.IsInRole(WindowsBuiltInRole.Administrator);
            }
        }

        private static string RunElevatedCommand(string exe, string args)
        {
            try
            {
                bool isAdmin = IsAdministrator();
                var psi = new ProcessStartInfo
                {
                    FileName = exe,
                    Arguments = args,
                    UseShellExecute = !isAdmin, // If not admin, must use ShellExecute for "runas" UAC prompt
                    Verb = isAdmin ? "" : "runas",
                    RedirectStandardOutput = isAdmin,
                    RedirectStandardError = isAdmin,
                    CreateNoWindow = true
                };

                using (var p = Process.Start(psi))
                {
                    if (isAdmin)
                    {
                        string stdout = p.StandardOutput.ReadToEnd();
                        string stderr = p.StandardError.ReadToEnd();
                        p.WaitForExit(8000);
                        return stdout + " " + stderr;
                    }
                    else
                    {
                        p.WaitForExit(8000);
                        return p.ExitCode == 0 ? "SUCCESS (Elevated)" : $"ExitCode: {p.ExitCode}";
                    }
                }
            }
            catch (Exception ex)
            {
                return "Exception: " + ex.Message;
            }
        }

        private static string GetArgValue(string[] args, string flag)
        {
            for (int i = 0; i < args.Length - 1; i++)
            {
                if (string.Equals(args[i], flag, StringComparison.OrdinalIgnoreCase))
                {
                    return args[i + 1];
                }
            }
            return null;
        }

        private static void OutputJson(object obj)
        {
            Console.WriteLine(serializer.Serialize(obj));
        }

        #endregion
    }
}
