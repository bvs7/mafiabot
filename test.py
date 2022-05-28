
from typing import Dict, Any
import json

"""
Encoding and decoding...
Encode in a simple, human readable way
Decode in a way that knows types?
If the constructor just initializes based on basic info,
and encoding just simplifies to basic...
"""

class MyDecoder(json.JSONDecoder):

    def __init__(self):
        json.JSONDecoder.__init__(self, object_hook=MyDecoder.object_hook)
        self.parse_array = MyDecoder.parse_array_

    def decode(self, s):
        print(f"Decoding... {s}")
        return super().decode(s)

    @staticmethod
    def parse_array_(s_and_end, scan_once):
        print(s_and_end, scan_once)
        return super().parse_array(s_and_end, scan_once)

    @staticmethod
    def object_hook(d : Dict[str,Any]):
        print(d)
        return d

s = json.dumps({"t":1,"s":[1,2,3,4,5],"r":3})

print(json.loads(s, cls=MyDecoder))
