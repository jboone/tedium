#!/usr/bin/env python3

import time

import usb.core

from tedium.gateware.usb.descriptors_vendor import Descriptors

INTERFACE = Descriptors.InterfaceNumber.FramerControl
ENDPOINT = Descriptors.EndpointNumber.FramerControl
ALTERNATE_SETTING = 0

dev = usb.core.find(
    idVendor=Descriptors.VENDOR_ID,
    idProduct=Descriptors.PRODUCT_ID
)

cfg = dev.get_active_configuration()
intf = cfg[(INTERFACE,ALTERNATE_SETTING)]
print(intf)
dev.set_interface_altsetting(
    interface=INTERFACE,
    alternate_setting=ALTERNATE_SETTING
)

ep = usb.util.find_descriptor(
    intf,
    custom_match=lambda e:
        usb.util.endpoint_direction(e.bEndpointAddress) == usb.util.ENDPOINT_OUT \
        and usb.util.endpoint_address(e.bEndpointAddress) == ENDPOINT
)

while True:
    try:
        ep.write(bytes([0x00, 0xfe, 0x01]))
        time.sleep(0.5)
    except usb.core.USBError as e:
        print(e)

