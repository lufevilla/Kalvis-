import 'dart:convert';
import 'package:http/http.dart' as http;
import 'package:flutter/material.dart';
import 'package:front_end/core/models/tracking.dart';

class TrackingApiService {
  final String baseUrl;

  TrackingApiService({this.baseUrl = 'http://localhost:3000'});

  Future<TrackingData> fetchTrackingData({String astronautId = 'AST001'}) async {
    final uri = Uri.parse('$baseUrl/tracking').replace(queryParameters: {
      'astronauta_id': astronautId,
    });

    final response = await http.get(uri);

    if (response.statusCode == 200) {
      final jsonData = json.decode(response.body);
      return TrackingData.fromJson(jsonData);
    } else {
      throw Exception('Failed to load tracking data: ${response.statusCode}');
    }
  }
}