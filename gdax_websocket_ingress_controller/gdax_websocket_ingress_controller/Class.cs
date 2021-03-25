using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

namespace gdax_websocket_ingress_controller
{
    public class ThreadSet
    {
        private Thread threadStore;
        private ParameterizedThreadStart threadStart;
        public ConcurrentQueue<string> rxbuffer { get; set; }
        public ConcurrentQueue<string> txbuffer { get; set; }
        public Process process { get; set; } 

        public ThreadSet(ParameterizedThreadStart passedThreadStart)
        {
            threadStart = passedThreadStart;
            threadStore = new Thread(threadStart);
            rxbuffer = new ConcurrentQueue<string>();
            txbuffer = new ConcurrentQueue<string>();
            process = null;
        }

        public void Start(object[] passedArgs)
        {
            threadStore.Start(passedArgs);
        }
        public void Join()
        {
            threadStore.Join();
        }

        public void Abort()
        {
            threadStore.Abort();
        }
    }

    public class status_subscribe
    {
        public string type { get; set; }
        public List<Dictionary<string, string>> channels { get; set; }
    }
    public class channels
    {
        public string name { get; set; }
    }

    public class websocket_type
    {
        public string type { get; set; }
    }

    public class websocket_status_currency_list
    {
        public string type { get; set; }
        public websocket_status_currency_list_currencies[] currencies { get; set; }
        public websocket_status_currency_list_products[] products { get; set; }
    }

    public class websocket_status_currency_list_currencies
    {
        public string id { get; set; }
        public string name { get; set; }
        public string min_size { get; set; }
        public string status { get; set; }
        public string funding_account_id { get; set; }
        public string status_message { get; set; }
        public Decimal max_precision { get; set; }
        public string[] convertible_to { get; set; }
        public websocket_status_currency_list_currencies_details details { get; set; }
    }

    public class websocket_status_currency_list_currencies_details
    {
        public string type { get; set; }
        public string symbol { get; set; }
        public int network_confirmations { get; set; }
        public int sort_order { get; set; }
        public string crypto_address_link { get; set; }
        public string crypto_transaction_link { get; set; }
        public string[] push_payment_methods { get; set; }
        public int processing_time_seconds { get; set; }
        public Decimal min_withdrawal_amount { get; set; }
        public Decimal max_withdrawal_amount { get; set; }
    }

    public class websocket_status_currency_list_products
    {
        public string id { get; set; }
        public string base_currency { get; set; }
        public string quote_currency { get; set; }
        public string base_min_size { get; set; }
        public string base_max_size { get; set; }
        public string base_increment { get; set; }
        public string quote_increment { get; set; }
        public string display_name { get; set; }
        public string status { get; set; }
        public bool margin_enabled { get; set; }
        public string status_message { get; set; }
        public string min_market_funds { get; set; }
        public string max_market_funds { get; set; }
        public bool post_only { get; set; }
        public bool limit_only { get; set; }
        public bool cancel_only { get; set; }
        public string type { get; set; }
    }
}
