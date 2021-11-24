# Removing `AckMode`

The `Ack` implementation wasn't particular useful under practical situations as the 
client would likely reconnect with a different port and would thus be treated as a
new client. 

As the long term goal is to switch to some protocol like AMQP (which is another 
project that I am currently working on), proper support for message delivery 
acknowledgement can be expected when that is ready.