var searchIndex = JSON.parse('{\
"sangfroid":{"doc":"ci-tests rustdoc","t":[0,0,0,0,3,11,11,11,11,11,12,11,12,12,11,11,11,11,4,13,13,11,11,11,11,11,11,11,12,13,13,13,13,13,3,4,13,13,12,11,11,11,11,11,12,11,11,11,11,11,11,11,11,11,12,5,11,11,11,11,11,11,11,11,11,5,5,13,13,13,13,13,3,4,11,11,11,11,11,11,12,11,11,11,11,11,11,11,11,11,11,11,11,12,11,11,11,11,11,11,11,11,11,12,12,11],"n":["job","message","threadpool","worker","Job","borrow","borrow_mut","from","into","new","req","resp_with_result","result_sink","task","try_from","try_into","type_id","with_result_sink","Message","Request","Terminate","borrow","borrow_mut","from","into","try_from","try_into","type_id","0","JobSchedulingFailed","JoinFailed","LockError","LookupError","TermNoticeFailed","ThreadPool","ThreadPoolError","WorkerTermFailed","WorkerUnavailable","balancer","balancer_thread","borrow","borrow","borrow_mut","borrow_mut","done_channel","drop","fmt","fmt","from","from","into","into","new","new_workers","pool","restore_worker_pool_order","schedule","terminate","to_string","try_from","try_from","try_into","try_into","type_id","type_id","worker_pool_schedule_job","worker_pool_terminate","DispatchFailed","DoneNotificationFailed","JoinFailed","ResultResponseFailed","TermNoticeFailed","Worker","WorkerError","borrow","borrow","borrow_mut","borrow_mut","cmp","dec_load","disp_q","dispatch","drop","eq","fmt","fmt","from","from","inc_load","into","into","new","partial_cmp","pending","terminate","to_string","try_from","try_from","try_into","try_into","type_id","type_id","uid","uid","worker","worker_thread"],"q":["sangfroid","","","","sangfroid::job","","","","","","","","","","","","","","sangfroid::message","","","","","","","","","","sangfroid::message::Message","sangfroid::threadpool","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","sangfroid::worker","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","",""],"d":["Jobs to be dispatched and executed.","Message entity for communicating with workers.","A load balanced threadpool.","Workers in a threadpool.","Represents a job to be submitted to the threadpool.","","","Returns the argument unchanged.","Calls <code>U::from(self)</code>.","Creates a new job from a closure and Req arguments to be …","request to service i.e args for task","Consumes this job and responds with the result computed.","Optional result channel to send the result of the execution","Task to be executed","","","","Creates a new job like <code>::new()</code> with a result sink for …","Message represents a message to be sent to workers in a …","Request for job execution","Message the thread to terminate itself.","","","Returns the argument unchanged.","Calls <code>U::from(self)</code>.","","","","","","","","","","ThreadPool to keep track of worker threads, dynamically …","","","","","Returns a <code>JoinHandle</code> to a balancer thread for the given …","","","","","","Invokes <code>terminate()</code>","","","Returns the argument unchanged.","Returns the argument unchanged.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Creates a new threadpool with the given number of workers …","Creates the given number of workers and returns them in a …","","Restores the order of the workers in the worker pool after …","Schedules a new job to the worker pool by picking up the …","Terminates this threadpool by invoking …","","","","","","","","Schedules a new job to the given worker pool by picking up …","Terminates all workers in the given pool of workers by …","","","","","","Worker represents a worker thread capable for receiving …","","","","","","Uses cmp().reverse() on pending tasks to favor Workers …","Decrements pending tasks by 1.","message dispatch queue","Dispatches a job to this worker for execution.","Invokes terminate()","Worker are considered equal if they have the same number …","","","Returns the argument unchanged.","Returns the argument unchanged.","Increments pending tasks by 1.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Creates a new Worker from the given source, dispatch queue …","","number of pending jobs to be serviced","Terminates this worker by sending a Terminate message to …","","","","","","","","","uid for uniquely identifying this worker","worker thread for executing jobs","Creates a worker thread from the given job source, done …"],"i":[0,0,0,0,0,1,1,1,1,1,1,1,1,1,1,1,1,1,0,2,2,2,2,2,2,2,2,2,3,4,4,4,4,4,0,0,4,4,5,5,5,4,5,4,5,5,4,4,5,4,5,4,5,5,5,0,5,5,4,5,4,5,4,5,4,0,0,6,6,6,6,6,0,0,7,6,7,6,7,7,7,7,7,7,6,6,7,6,7,7,6,7,7,7,7,6,7,6,7,6,7,6,7,7,7,7],"f":[null,null,null,null,null,[[["",0]],["",0]],[[["",0]],["",0]],[[]],[[]],[[]],null,[[],["result",4,[["senderror",3]]]],null,null,[[],["result",4]],[[],["result",4]],[[["",0]],["typeid",3]],[[]],null,null,null,[[["",0]],["",0]],[[["",0]],["",0]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[["",0]],["typeid",3]],null,null,null,null,null,null,null,null,null,null,null,[[["receiver",3,[["option",4,[["u64",0]]]]],["arc",3,[["mutex",3,[["binarymaxheap",3,[["worker",3]]]]]]]],["joinhandle",3,[["result",4,[["threadpoolerror",4]]]]]],[[["",0]],["",0]],[[["",0]],["",0]],[[["",0]],["",0]],[[["",0]],["",0]],null,[[["",0]]],[[["",0],["formatter",3]],["result",6]],[[["",0],["formatter",3]],["result",6]],[[]],[[]],[[]],[[]],[[["usize",0]]],[[["usize",0]]],null,[[["binarymaxheap",3],["u64",0]],["result",4,[["threadpoolerror",4]]]],[[["",0],["job",3]],["result",4,[["threadpoolerror",4]]]],[[["",0]],["result",4,[["threadpoolerror",4]]]],[[["",0]],["string",3]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[["",0]],["typeid",3]],[[["",0]],["typeid",3]],[[["binarymaxheap",3],["job",3]],["result",4,[["threadpoolerror",4]]]],[[["binarymaxheap",3]],["result",4,[["threadpoolerror",4]]]],null,null,null,null,null,null,null,[[["",0]],["",0]],[[["",0]],["",0]],[[["",0]],["",0]],[[["",0]],["",0]],[[["",0],["",0]],["ordering",4]],[[["",0]]],null,[[["",0],["job",3]],["result",4,[["workererror",4]]]],[[["",0]]],[[["",0],["",0]],["bool",0]],[[["",0],["formatter",3]],["result",6]],[[["",0],["formatter",3]],["result",6]],[[]],[[]],[[["",0]]],[[]],[[]],[[["receiver",3,[["message",4]]],["sender",3,[["message",4]]],["sender",3,[["option",4,[["u64",0]]]]],["u64",0]]],[[["",0],["",0]],["option",4,[["ordering",4]]]],null,[[["",0]],["result",4,[["workererror",4]]]],[[["",0]],["string",3]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[["",0]],["typeid",3]],[[["",0]],["typeid",3]],[[["",0]],["u64",0]],null,null,[[["receiver",3,[["message",4]]],["sender",3,[["option",4,[["u64",0]]]]],["u64",0]],["joinhandle",3,[["result",4,[["workererror",4]]]]]]],"p":[[3,"Job"],[4,"Message"],[13,"Request"],[4,"ThreadPoolError"],[3,"ThreadPool"],[4,"WorkerError"],[3,"Worker"]]}\
}');
if (window.initSearch) {window.initSearch(searchIndex)};