/*
 * This file is part of Tedium.
 *
 * Copyright (C) 2020 Jared Boone <jared@sharebrained.com>
 * SPDX-License-Identifier: BSD-3-Clause
 */

#include <cstdio>
#include <cstring>
#include <unistd.h>
#include <byteswap.h>

#include <libusb-1.0/libusb.h>

const uint16_t VENDOR_ID  = 0x16d0;
const uint16_t PRODUCT_ID = 0x0f3b;

const uint8_t ISO_INTERFACE = 0;
const uint8_t ISO_ALT_SETTING_ACTIVE = 1;

const uint8_t ISO_IN_ENDPOINT_NUMBER = 1;
const uint8_t ISO_IN_ENDPOINT_ADDRESS = ISO_IN_ENDPOINT_NUMBER | LIBUSB_ENDPOINT_IN;
const unsigned int ISO_IN_TIMEOUT = 1000;
const int NUM_ISO_IN_PACKETS = 8;
const unsigned int FRAME_LENGTH_IN = 512;

const uint8_t ISO_OUT_ENDPOINT_NUMBER = 1;
const uint8_t ISO_OUT_ENDPOINT_ADDRESS = ISO_OUT_ENDPOINT_NUMBER | LIBUSB_ENDPOINT_OUT;
const unsigned int ISO_OUT_TIMEOUT = 1000;
const int NUM_ISO_OUT_PACKETS = 8;
const unsigned int FRAME_LENGTH_OUT = 512;

// It seems that having a lot of ISO packets in reserve helps avoid dropped bits.
// I'm still not sure what cranking up the ISO packets gets you vs. an increased
// number of transfers.
const size_t NUM_TRANSFERS = 8;

static uint8_t last = 0;

static int64_t iso_in_byte_counter = 0;
static int64_t iso_out_byte_counter = 0;

void callback_iso_in(libusb_transfer* transfer) {
	// printf("IN: callback: %d: %d\n", transfer->status, transfer->iso_packet_desc[0].actual_length);

	static uint8_t iso_in_expected_byte = 0;

	// "If this is an isochronous transfer, this field may read COMPLETED even if there were errors in the frames. Use the status field in each packet to determine if errors occurred."
	if( transfer->status == LIBUSB_TRANSFER_COMPLETED ) {
		for(auto i=0; i<NUM_ISO_IN_PACKETS; i++) {
			auto packet = &transfer->iso_packet_desc[i];

			if( packet->status == LIBUSB_TRANSFER_COMPLETED) {
				switch(packet->actual_length) {
					case 0:
					case 211 * 0 + 12:
					case 211 * 1 + 12:
					case 211 * 2 + 12: {
							auto b = libusb_get_iso_packet_buffer(transfer, i);
							if (b != NULL) {
								// iso_in_buffer_count += 1;
								// if(iso_in_buffer_count % 80000 == 0) {
								// 	printf("IN: %lu\n", iso_in_buffer_count);
								// }

								for(auto j=0; j<packet->actual_length; j++) {
									if(b[j] != iso_in_expected_byte) {
										printf("IN: packet[%2d] %02x != %02x\n", i, b[j], iso_in_expected_byte);
										iso_in_expected_byte = b[j];
									}
									iso_in_expected_byte += 1;
								}

								iso_in_byte_counter += packet->actual_length;

								// printf("IN: packet[%2d] length=%u\n", i, packet->actual_length);
							} else {
								printf("IN: packet[%2d] libusb_get_iso_packet_buffer_simple(transfer) returned NULL\n", i);
							}
						}
						break;

					default: {
							printf("IN: packet %2d incomplete, length %3d\n", i, packet->actual_length);
							break;
						}
				}
			} else {
				printf("IN: packet[%2d] status = %d\n", i, packet->status);
			}
		}
	} else {
		printf("IN: transfer status = %d\n", transfer->status);
	}

	auto result = libusb_submit_transfer(transfer);
	if( result != 0 ) {
		printf("IN: libusb_submit_transfer failed: %d\n", result);
	}
}

void callback_iso_out(libusb_transfer* transfer) {
	// printf("OUT: callback: %d: %d\n", transfer->status, transfer->iso_packet_desc[0].actual_length);

	static size_t iso_out_buffer_counter = 0;

	libusb_set_iso_packet_lengths(transfer, 53);

	for(auto i=0; i<NUM_ISO_OUT_PACKETS; i++) {
		auto packet = &transfer->iso_packet_desc[i];
		auto b = libusb_get_iso_packet_buffer(transfer, i);

		iso_out_buffer_counter += 1;
		if(iso_out_buffer_counter % 80000 == 0) {
			printf("OUT=%8ld IN=%8ld diff=%ld\n", iso_out_byte_counter, iso_in_byte_counter, iso_in_byte_counter - iso_out_byte_counter);
		}

		for(auto j=0; j<packet->length; j++) {
			b[j] = iso_out_byte_counter;
			iso_out_byte_counter += 1;
		}

		// memset(b, v, packet->length);

		// F bits.
		// b[0] = 0xff;

		// if (packet->length != packet->actual_length) {
		// 	// TODO: If `actual_length` is zero, does that mean I should resubmit this packet for transmission?
		// 	// Will things get out of order?
		// 	printf("OUT: packet[%d] length=%d actual_length=%d\n", i, packet->length, packet->actual_length);
		// }
	}

	auto result = libusb_submit_transfer(transfer);
	if( result != 0 ) {
		printf("OUT: libusb_submit_transfer failed: %d\n", result);
	}
}

int main(int argc, char ** argv) {
	libusb_context* context = NULL;
	if( libusb_init(&context) != 0 ) {
		printf("libusb_init failed\n");
		return -2;
	}

	auto device_handle = libusb_open_device_with_vid_pid(context, VENDOR_ID, PRODUCT_ID);
	if( device_handle == NULL ) {
		printf("libusb_open_device_with_vid_pid failed\n");
		return -3;
	}

	auto result = libusb_claim_interface(device_handle, ISO_INTERFACE);
	if( result != 0 ) {
		printf("IN: libusb_claim_interface failed: %d\n", result);
		return -4;
	}

	result = libusb_set_interface_alt_setting(device_handle, ISO_INTERFACE, ISO_ALT_SETTING_ACTIVE);
	if( result != 0 ) {
		printf("IN: libusb_set_interface_alt_setting: %d\n", result);
		return -5;
	}

	// ISO IN transfers
	for(auto i=0; i<NUM_TRANSFERS; i++) {
		auto transfer = libusb_alloc_transfer(NUM_ISO_IN_PACKETS);
		if( transfer == NULL ) {
			printf("IN: libusb_alloc_transfer failed\n");
			return -6;
		}

		const size_t BUFFER_LENGTH = FRAME_LENGTH_IN * NUM_ISO_IN_PACKETS;
		auto buffer = new uint8_t[BUFFER_LENGTH];
		libusb_fill_iso_transfer(transfer, device_handle, ISO_IN_ENDPOINT_ADDRESS, buffer, BUFFER_LENGTH, NUM_ISO_IN_PACKETS, callback_iso_in, NULL, ISO_IN_TIMEOUT);

		libusb_set_iso_packet_lengths(transfer, FRAME_LENGTH_IN);

		result = libusb_submit_transfer(transfer);
		if( result != 0 ) {
			printf("IN: libusb_submit_transfer failed: %d\n", result);
			return -7;
		}
	}

	// ISO OUT transfers
	for(auto i=0; i<NUM_TRANSFERS; i++) {
		auto transfer = libusb_alloc_transfer(NUM_ISO_OUT_PACKETS);
		if( transfer == NULL ) {
			printf("OUT: libusb_alloc_transfer failed\n");
			return -10;
		}

		const size_t BUFFER_LENGTH = FRAME_LENGTH_OUT * NUM_ISO_OUT_PACKETS;
		auto buffer = new uint8_t[BUFFER_LENGTH];
		libusb_fill_iso_transfer(transfer, device_handle, ISO_OUT_ENDPOINT_ADDRESS, buffer, BUFFER_LENGTH, NUM_ISO_OUT_PACKETS, callback_iso_out, NULL, ISO_OUT_TIMEOUT);

		libusb_set_iso_packet_lengths(transfer, FRAME_LENGTH_OUT);

		for(auto i=0; i<NUM_ISO_OUT_PACKETS; i++) {
			auto packet = &transfer->iso_packet_desc[i];

			printf("OUT: packet[%d] length=%d actual_length=%d\n", i, packet->length, packet->actual_length);
		}

		result = libusb_submit_transfer(transfer);
		if( result != 0 ) {
			printf("OUT: libusb_submit_transfer failed: %d\n", result);
			return -11;
		}
	}

	while(true) {
		result = libusb_handle_events(context);
		if( result != 0 ) {
			printf("libusb_handle_events failed: %d\n", result);
			return -12;
		}
	}

	libusb_exit(context);

	return 0;
}
