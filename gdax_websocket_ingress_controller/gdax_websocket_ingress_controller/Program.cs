using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using System.Diagnostics;

using WebSocketSharp;
using Newtonsoft.Json;
using StackExchange.Redis;

namespace gdax_websocket_ingress_controller
{
    class Program
    {
        static string wsHost = "ws://127.0.0.1:80";
        static DateTime wsCurrencyUpdate = DateTime.UtcNow;
        static List<string> wsCurrencies = new List<string>();
        static Dictionary<string, ThreadSet> Workers = new Dictionary<string, ThreadSet>();
        static uint wsID = 10;

        static void Main(string[] args)
        {
            // Some extra debug
            Console.WriteLine("[main] Starting application");

            // Create a GC as we always seem to need one
            Workers.Add("gc", new ThreadSet(new ParameterizedThreadStart(EntryPoint)));
            Workers["gc"].Start(new object[] { "gc" });

            // Begin a runner for currency list polling
            Workers.Add("currencypoller", new ThreadSet(new ParameterizedThreadStart(EntryPoint)));
            Workers["currencypoller"].Start(new object[] { "currencypoller" });

            // Wait for the initial currency update
            while (wsCurrencies.Count == 0)
            {
                Thread.Sleep(2000);
            }

            Console.WriteLine("[main] Currency list loaded, proceeding");


            // Create a process runner
            Workers.Add("webrunner1", new ThreadSet(new ParameterizedThreadStart(EntryPoint)));
            Workers["webrunner1"].Start(new object[] { "webrunner1" });
            Workers.Add("webrunner2", new ThreadSet(new ParameterizedThreadStart(EntryPoint)));
            Workers["webrunner2"].Start(new object[] { "webrunner2" });
            Workers.Add("webrunner3", new ThreadSet(new ParameterizedThreadStart(EntryPoint)));
            Workers["webrunner3"].Start(new object[] { "webrunner3" });

            // Scan its buffer see if we get anything
            string readdata = string.Empty;
            while (true)
            {
                Thread.Sleep(1);
            }
        }

        private static void EntryPoint(object passedObjs)
        {
            // Processed passed objects
            object[] passedObjs1 = (object[])passedObjs;
            string name = (string)passedObjs1[0];

            // Begin entry processing
            if (name.Equals("gc"))
            {
                beginGC();
            }
            else if (name.Equals("currencypoller"))
            {
                bool runPoll = true;
                while (true)
                {
                    if (wsCurrencyUpdate.AddMinutes(30) < DateTime.UtcNow)
                    {
                        runPoll = true;
                    }
                    if (runPoll)
                    {
                        runCurrencyPolling();
                        runPoll = false;
                    }
                    Thread.Sleep(10000);
                }
            }
            else if (name.StartsWith("webrunner"))
            {
                beginWebRunner(name);
            }
            else
            {
                throw new NotImplementedException(string.Format("Thread start with no entrypoint: {0}", name));
            }

            if (name.StartsWith("webrunner"))
            {
                Workers.Remove(name);

                string workerName = string.Format("webrunner{0}", wsID);
                wsID += 1;
                
                Workers.Add(workerName, new ThreadSet(new ParameterizedThreadStart(EntryPoint)));
                Workers[workerName].Start(new object[] { workerName });
            }
                
        }

        private static void beginWebRunner(string threadName)
        {
            ThreadSet threadStash = Workers[threadName];
            Console.WriteLine("[WebRunner] Process starting");

            // Connection to redis
            ConnectionMultiplexer muxer = ConnectionMultiplexer.Connect("localhost");
            IDatabase conn = muxer.GetDatabase();

            StringBuilder outputBuilder = new StringBuilder();

            var processStartInfo = new ProcessStartInfo
            {
                FileName = @"/home/paul.webster/dev/rust/gdax_portal/target/release/gdax_portal",
                RedirectStandardOutput = true,
                UseShellExecute = false
            };

            threadStash.process = new Process();
            threadStash.process.StartInfo = processStartInfo;
            threadStash.process.OutputDataReceived += new DataReceivedEventHandler
            (
                delegate (object sender, DataReceivedEventArgs e)
                {
                    // If we detect and error kill ourself
                    if (e.Data.StartsWith("WSB RX ERROR MAIN NoData"))
                    {
                        Console.WriteLine("[WebRunner] Killing: {0}", e.Data);
                        threadStash.process.CancelOutputRead();
                        threadStash.process.Kill();
                    }
                    else if (e.Data.StartsWith("WSB RX DATA "))
                    {
                        string[] cols = e.Data.Split(new char[] { ' ' },5);
                        string pkey = cols[3];
                        string json = cols[4];

                        conn.StringSet(pkey, json, flags: CommandFlags.FireAndForget);
                    }
                    else
                    {
                        Console.WriteLine("[WebRunner] {0}", e.Data);
                    }

                }
            );

            try
            {
                // Kick the process into action
                threadStash.process.Start();
                threadStash.process.BeginOutputReadLine();

                // We likely will never hit these
                threadStash.process.WaitForExit();
                threadStash.process.CancelOutputRead();
            }
            catch (Exception exception)
            {
                Console.WriteLine("[WebRunner] Exception: {0}", exception.Message);
            }
        }

        private static void runCurrencyPolling()
        {
            WebSocket wsClient = new WebSocket(wsHost);

            wsClient.OnOpen += (sender, e) =>
            {
                status_subscribe subscribe_object = new status_subscribe();
                subscribe_object.type = "subscribe";
                subscribe_object.channels = new List<Dictionary<string, string>>();
                subscribe_object.channels.Add(new Dictionary<string, string>() { { "name", "status" } });

                string subscribe_json = JsonConvert.SerializeObject(subscribe_object);

                wsClient.Send(subscribe_json);
            };
            wsClient.OnError += (sender, e) =>
            {
                wsClient = null;
                return;
            };
            wsClient.OnClose += (sender, e) =>
            {

                wsClient = null;
                return;
            };
            wsClient.OnMessage += (sender, e) =>
            {
                string jsonAsString = e.Data;
                websocket_type packet = JsonConvert.DeserializeObject<websocket_type>(jsonAsString);
                string type = packet.type;

                if (!type.Equals("status"))
                {
                    Console.WriteLine("[CurencyPoll] Ignoring packet type: {0}", type);
                    return;
                }

                websocket_status_currency_list currencyList =
                    JsonConvert.DeserializeObject<websocket_status_currency_list>(jsonAsString);

                List<string> currencyBuilder = new List<string>();
                foreach (websocket_status_currency_list_products product_detail in currencyList.products)
                {
                    if (!product_detail.status.Equals("online"))
                    {
                        continue;
                    }
                    currencyBuilder.Add(product_detail.id);
                }


                // Compare the stored currencyList vs the remote list
                List<string> listDiff1 = wsCurrencies.Except(currencyBuilder).ToList();
                List<string> listDiff2 = currencyBuilder.Except(wsCurrencies).ToList();

                // Only update if a difference was found one way or another
                if (listDiff1.Count > 0 || listDiff2.Count > 0)
                {
                    // Expensive but we do not need to do it often
                    lock (wsCurrencies)
                    {
                        wsCurrencies.Clear();
                        wsCurrencies = currencyBuilder;
                    };
                    Console.WriteLine("[CurencyPoll] Availible currencies changed, updated.");
                }
                else
                {
                    Console.WriteLine("[CurencyPoll] Availible currencies have not changed.");
                }

                wsClient.Close();
            };

            wsClient.Connect();

            while (wsClient != null && wsClient.IsAlive)
            {
                Thread.Sleep(1000);
            }
        }

        private static void beginGC()
        {
            while (true)
            {
                Thread.Sleep(1000);
            }
        }

        private static DateTime ConvertFromUnixTimestamp(double timestamp)
        {
            DateTime origin = new DateTime(1970, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc);
            return origin.AddSeconds(timestamp);
        }

        private static double ConvertToUnixTimestamp(DateTime date)
        {
            DateTime origin = new DateTime(1970, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc);
            TimeSpan diff = date.ToUniversalTime() - origin;
            return Math.Floor(diff.TotalSeconds);
        }
    }
}
